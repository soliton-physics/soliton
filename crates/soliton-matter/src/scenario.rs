//! The catalogue: class 11 properties of matter, chapter by chapter (design 018).

use crate::fluids::{Capillary, Depth, Draining, Falling, Floating, Lift, Venturi};
use crate::heat::{Conducting, Cooling, Expanding, Glowing, Heating, Mixing};
use crate::picture::{Answer, Lesson, Picture, Plot, Reading};
use crate::solids::{Sheared, Squeezed, Wire};
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

const SOLIDS: &str = "Mechanical properties of solids";
const FLUIDS: &str = "Mechanical properties of fluids";
const HEAT: &str = "Thermal properties of matter";
const GIGA: f64 = 1e9;

const fn syllabus(chapter: &'static str, topic: &'static str) -> Syllabus {
    Syllabus {
        class: 11,
        chapter,
        topic,
    }
}

static WIRE: [Param; 5] = [
    param(
        "load",
        "Load hung on the wire",
        "kg",
        (0.0, 60.0),
        0.5,
        10.0,
    ),
    param(
        "diameter",
        "Diameter of the wire",
        "mm",
        (0.2, 3.0),
        0.05,
        1.0,
    ),
    param("length", "Length of the wire", "m", (0.5, 5.0), 0.1, 2.0),
    param(
        "youngs",
        "Young's modulus",
        "GPa",
        (50.0, 220.0),
        5.0,
        200.0,
    ),
    param("limit", "Elastic limit", "MPa", (100.0, 500.0), 10.0, 250.0),
];

static SHEAR: [Param; 4] = [
    param(
        "force",
        "Force along the top face",
        "kN",
        (0.0, 200.0),
        1.0,
        90.0,
    ),
    param("modulus", "Shear modulus", "GPa", (5.0, 90.0), 0.1, 5.6),
    param(
        "area",
        "Area of the top face",
        "cm²",
        (50.0, 1000.0),
        10.0,
        500.0,
    ),
    param(
        "height",
        "Height of the block",
        "cm",
        (5.0, 100.0),
        1.0,
        50.0,
    ),
];

static SQUEEZE: [Param; 2] = [
    param(
        "depth",
        "Depth in the sea",
        "m",
        (0.0, 11_000.0),
        100.0,
        3000.0,
    ),
    param("modulus", "Bulk modulus", "GPa", (1.0, 200.0), 0.1, 2.2),
];

static DEPTH: [Param; 2] = [
    param("depth", "Depth of the gauge", "m", (0.0, 50.0), 0.5, 10.0),
    param(
        "density",
        "Density of the liquid",
        "kg/m³",
        (700.0, 13_600.0),
        10.0,
        1000.0,
    ),
];

static LIFT: [Param; 3] = [
    param(
        "force",
        "Force on the small piston",
        "N",
        (10.0, 500.0),
        10.0,
        100.0,
    ),
    param(
        "small",
        "Diameter of the small piston",
        "cm",
        (1.0, 10.0),
        0.5,
        2.0,
    ),
    param(
        "large",
        "Diameter of the large piston",
        "cm",
        (5.0, 60.0),
        1.0,
        20.0,
    ),
];

static FLOAT: [Param; 2] = [
    param(
        "block",
        "Density of the block",
        "kg/m³",
        (100.0, 3000.0),
        10.0,
        600.0,
    ),
    param(
        "liquid",
        "Density of the liquid",
        "kg/m³",
        (700.0, 1400.0),
        10.0,
        1000.0,
    ),
];

static FLOW: [Param; 3] = [
    param(
        "speed",
        "Speed in the wide part",
        "m/s",
        (0.5, 4.0),
        0.1,
        1.5,
    ),
    param(
        "ratio",
        "Area of the pipe over area of the throat",
        "",
        (1.5, 5.0),
        0.1,
        3.0,
    ),
    param(
        "density",
        "Density of the liquid",
        "kg/m³",
        (700.0, 1300.0),
        10.0,
        1000.0,
    ),
];

static DRAIN: [Param; 4] = [
    param("level", "Water above the hole", "m", (0.2, 3.0), 0.05, 1.0),
    param(
        "height",
        "Height of the hole above the ground",
        "m",
        (0.2, 2.0),
        0.05,
        1.0,
    ),
    param("hole", "Diameter of the hole", "cm", (0.5, 5.0), 0.1, 2.0),
    param("tank", "Diameter of the tank", "m", (0.2, 1.0), 0.05, 0.4),
];

static FALL: [Param; 4] = [
    param("radius", "Radius of the ball", "mm", (0.5, 5.0), 0.1, 2.0),
    param(
        "ball",
        "Density of the ball",
        "kg/m³",
        (1400.0, 12_000.0),
        100.0,
        7800.0,
    ),
    param(
        "liquid",
        "Density of the liquid",
        "kg/m³",
        (800.0, 1300.0),
        10.0,
        1260.0,
    ),
    param(
        "viscosity",
        "Viscosity of the liquid",
        "Pa s",
        (0.05, 2.0),
        0.01,
        0.83,
    ),
];

static TUBE: [Param; 4] = [
    param(
        "radius",
        "Radius of the narrowest tube",
        "mm",
        (0.1, 2.0),
        0.05,
        0.5,
    ),
    param(
        "tension",
        "Surface tension",
        "mN/m",
        (20.0, 480.0),
        1.0,
        73.0,
    ),
    param("angle", "Angle of contact", "°", (0.0, 150.0), 1.0, 0.0),
    param(
        "density",
        "Density of the liquid",
        "kg/m³",
        (800.0, 13_600.0),
        10.0,
        1000.0,
    ),
];

static ROD: [Param; 3] = [
    param("rise", "Rise in temperature", "K", (0.0, 200.0), 1.0, 50.0),
    param(
        "alpha",
        "Coefficient of linear expansion",
        "× 10⁻⁶ /K",
        (1.0, 30.0),
        0.1,
        12.0,
    ),
    param("length", "Length of the rod", "m", (0.5, 100.0), 0.5, 10.0),
];

static MIX: [Param; 5] = [
    param(
        "block_temperature",
        "Temperature of the block",
        "°C",
        (30.0, 300.0),
        1.0,
        100.0,
    ),
    param(
        "block_mass",
        "Mass of the block",
        "g",
        (50.0, 1000.0),
        10.0,
        200.0,
    ),
    param(
        "specific_heat",
        "Specific heat of the block",
        "J/kg K",
        (100.0, 1000.0),
        5.0,
        900.0,
    ),
    param(
        "water_mass",
        "Mass of the water",
        "g",
        (50.0, 1000.0),
        10.0,
        300.0,
    ),
    param(
        "water_temperature",
        "Temperature of the water",
        "°C",
        (0.0, 60.0),
        1.0,
        20.0,
    ),
];

static HEATING: [Param; 3] = [
    param(
        "power",
        "Power of the heater",
        "W",
        (100.0, 2000.0),
        50.0,
        500.0,
    ),
    param("mass", "Mass of ice", "g", (50.0, 500.0), 10.0, 100.0),
    param(
        "start",
        "Temperature of the ice at the start",
        "°C",
        (-40.0, 0.0),
        1.0,
        -20.0,
    ),
];

static BAR: [Param; 6] = [
    param(
        "hot",
        "Temperature of the hot end",
        "°C",
        (50.0, 200.0),
        5.0,
        100.0,
    ),
    param(
        "cold",
        "Temperature of the cold end",
        "°C",
        (0.0, 40.0),
        1.0,
        0.0,
    ),
    param(
        "first",
        "Conductivity of the first metal",
        "W/m K",
        (10.0, 400.0),
        5.0,
        385.0,
    ),
    param(
        "second",
        "Conductivity of the second metal",
        "W/m K",
        (10.0, 400.0),
        5.0,
        50.0,
    ),
    param("length", "Length of the rod", "m", (0.2, 2.0), 0.1, 1.0),
    param(
        "area",
        "Area of its cross-section",
        "cm²",
        (1.0, 20.0),
        0.5,
        5.0,
    ),
];

static CUP: [Param; 3] = [
    param(
        "start",
        "Temperature at the start",
        "°C",
        (40.0, 100.0),
        1.0,
        90.0,
    ),
    param(
        "room",
        "Temperature of the room",
        "°C",
        (0.0, 40.0),
        1.0,
        25.0,
    ),
    param(
        "rate",
        "Cooling constant",
        "per min",
        (0.02, 0.5),
        0.01,
        0.1,
    ),
];

static GLOW: [Param; 1] = [param(
    "temperature",
    "Temperature",
    "K",
    (500.0, 10_000.0),
    50.0,
    5800.0,
)];

static CATALOGUE: &[Scenario] = &[
    Scenario {
        spec: ScenarioSpec {
            id: "stress-strain",
            title: "Stretching a wire",
            description: "Hang a load on a wire and it stretches. Up to a point the stretch is in proportion to the load, and the wire springs back when the load is taken off: Hooke's law. Go past the elastic limit and it gives way much more easily and stays stretched. Go further and it breaks.",
            syllabus: syllabus(SOLIDS, "Stress, strain and Young's modulus"),
            params: &WIRE,
        },
        build: |s, p| Box::new(Wire { youngs: p.get(s, "youngs") * GIGA, length: p.get(s, "length"), diameter: p.get(s, "diameter") * 1e-3, load: p.get(s, "load"), elastic_limit: p.get(s, "limit") * 1e6 }),
    },
    Scenario {
        spec: ScenarioSpec {
            id: "shear-modulus",
            title: "Shearing a block",
            description: "A force along the top face of a block, with the bottom held, pushes the top sideways and leans the sides over. The stress is the force over the area of the face it runs along, and the strain is how far the top moves over the height of the block. Solids resist this much less than they resist stretching.",
            syllabus: syllabus(SOLIDS, "Shear modulus"),
            params: &SHEAR,
        },
        build: |s, p| Box::new(Sheared { modulus: p.get(s, "modulus") * GIGA, force: p.get(s, "force") * 1e3, area: p.get(s, "area") * 1e-4, height: p.get(s, "height") * 1e-2 }),
    },
    Scenario {
        spec: ScenarioSpec {
            id: "bulk-modulus",
            title: "Squeezing from all sides",
            description: "Take something deep into the sea and the water presses on it from every side. Its volume shrinks by a fraction equal to the pressure over its bulk modulus. Water itself is squeezed by a little over one percent at three kilometres down; steel hardly at all.",
            syllabus: syllabus(SOLIDS, "Bulk modulus"),
            params: &SQUEEZE,
        },
        build: |s, p| Box::new(Squeezed { modulus: p.get(s, "modulus") * GIGA, depth: p.get(s, "depth") }),
    },
    Scenario {
        spec: ScenarioSpec {
            id: "pressure-depth",
            title: "Pressure and depth",
            description: "The deeper you go in a liquid, the more of it is above you pressing down. The pressure grows by the same amount for every metre, and by more in a denser liquid. Ten metres of water adds about as much as the whole atmosphere above it.",
            syllabus: syllabus(FLUIDS, "Variation of pressure with depth"),
            params: &DEPTH,
        },
        build: |s, p| Box::new(Depth { depth: p.get(s, "depth"), density: p.get(s, "density") }),
    },
    Scenario {
        spec: ScenarioSpec {
            id: "hydraulic-lift",
            title: "Pascal's law and the hydraulic lift",
            description: "Push on a liquid that is shut in and the extra pressure appears everywhere in it. A small force on a small piston therefore becomes a large force on a large one. Nothing is had for nothing: the large piston moves a correspondingly small distance.",
            syllabus: syllabus(FLUIDS, "Pascal's law and hydraulic machines"),
            params: &LIFT,
        },
        build: |s, p| Box::new(Lift::new(p.get(s, "force"), p.get(s, "small") * 1e-2, p.get(s, "large") * 1e-2)),
    },
    Scenario {
        spec: ScenarioSpec {
            id: "buoyancy",
            title: "Archimedes' principle: floating and sinking",
            description: "A block let go in a liquid is pushed up with a force equal to the weight of the liquid it shoves aside. If the block is the less dense, it sinks in only until that upthrust equals its weight, and floats with the rest of it above the surface. If it is the denser, the upthrust is never enough and it goes to the bottom.",
            syllabus: syllabus(FLUIDS, "Archimedes' principle"),
            params: &FLOAT,
        },
        build: |s, p| Box::new(Floating::new(p.get(s, "block"), p.get(s, "liquid"))),
    },
    Scenario {
        spec: ScenarioSpec {
            id: "bernoulli",
            title: "Bernoulli's principle in a Venturi tube",
            description: "Liquid flowing through a pipe has to go faster where the pipe is narrow, since the same amount passes every point each second. Where it goes faster its pressure is lower: the gauge over the throat stands lower than the gauge over the wide part. Watch a speck speed up through the throat and slow again.",
            syllabus: syllabus(FLUIDS, "Equation of continuity and Bernoulli's principle"),
            params: &FLOW,
        },
        build: |s, p| Box::new(Venturi::new(p.get(s, "speed"), p.get(s, "ratio"), p.get(s, "density"))),
    },
    Scenario {
        spec: ScenarioSpec {
            id: "torricelli",
            title: "Torricelli's law: a tank emptying",
            description: "Water leaves a hole in the side of a tank as fast as if it had fallen freely from the surface to the hole. As the tank empties the jet slackens and lands nearer. The level falls quickly at first and ever more slowly.",
            syllabus: syllabus(FLUIDS, "Speed of efflux"),
            params: &DRAIN,
        },
        build: |s, p| Box::new(Draining::new(p.get(s, "level"), p.get(s, "height"), p.get(s, "hole") * 1e-2, p.get(s, "tank"))),
    },
    Scenario {
        spec: ScenarioSpec {
            id: "stokes-law",
            title: "Viscosity and terminal velocity",
            description: "A ball dropped into a thick liquid speeds up for an instant and then falls steadily. The drag of the liquid grows with the ball's speed until, with the upthrust, it balances the weight. That steady speed is the terminal velocity, and it is how viscosity is measured.",
            syllabus: syllabus(FLUIDS, "Viscosity and Stokes' law"),
            params: &FALL,
        },
        build: |s, p| Box::new(Falling::new(p.get(s, "radius") * 1e-3, p.get(s, "ball"), p.get(s, "liquid"), p.get(s, "viscosity"))),
    },
    Scenario {
        spec: ScenarioSpec {
            id: "capillary-rise",
            title: "Surface tension and capillary rise",
            description: "Water climbs a fine glass tube, and climbs higher the finer the tube. Its surface pulls on the glass it wets, and that pull holds up the column. Mercury, which does not wet glass, is pushed down instead.",
            syllabus: syllabus(FLUIDS, "Surface tension, drops and capillary rise"),
            params: &TUBE,
        },
        build: |s, p| Box::new(Capillary { radius: p.get(s, "radius") * 1e-3, tension: p.get(s, "tension") * 1e-3, angle: p.get(s, "angle"), density: p.get(s, "density") }),
    },
    Scenario {
        spec: ScenarioSpec {
            id: "thermal-expansion",
            title: "Thermal expansion",
            description: "Nearly everything grows when it is heated. A rod gets longer by a fraction that is the same for every degree, and different for each material. The fraction is small, a hundredth of a percent or so, but a bridge or a railway line has to be given room for it.",
            syllabus: syllabus(HEAT, "Thermal expansion"),
            params: &ROD,
        },
        build: |s, p| Box::new(Expanding { alpha: p.get(s, "alpha") * 1e-6, length: p.get(s, "length"), rise: p.get(s, "rise") }),
    },
    Scenario {
        spec: ScenarioSpec {
            id: "calorimetry",
            title: "Calorimetry: a hot block in cool water",
            description: "Drop a hot block into cool water in a vessel that lets no heat escape. Heat goes from the block to the water until they are at one temperature. What the block loses the water gains, and that fixes where they meet. The water, with its large specific heat, usually moves the less.",
            syllabus: syllabus(HEAT, "Specific heat capacity and calorimetry"),
            params: &MIX,
        },
        build: |s, p| Box::new(Mixing::new(p.get(s, "block_mass") * 1e-3, p.get(s, "specific_heat"), p.get(s, "block_temperature"), p.get(s, "water_mass") * 1e-3, p.get(s, "water_temperature"))),
    },
    Scenario {
        spec: ScenarioSpec {
            id: "heating-curve",
            title: "Change of state: heating ice to steam",
            description: "Heat ice steadily and its temperature climbs, then stops at 0 °C while it melts, climbs again as water, stops at 100 °C while it boils, and climbs again as steam. While it is changing state the heat goes into pulling the molecules apart, not into making them move faster. Boiling takes by far the most.",
            syllabus: syllabus(HEAT, "Change of state and latent heat"),
            params: &HEATING,
        },
        build: |s, p| Box::new(Heating::new(p.get(s, "mass") * 1e-3, p.get(s, "start"), p.get(s, "power"))),
    },
    Scenario {
        spec: ScenarioSpec {
            id: "heat-conduction",
            title: "Conduction along a rod",
            description: "One end of a rod is kept hot and the other cold. Heat creeps along it until the temperature falls steadily from one end to the other. With two metals end to end the same heat must cross both, so the poorer conductor takes the steeper fall, and the join sits nearer the temperature of the end the good conductor is on.",
            syllabus: syllabus(HEAT, "Heat transfer: conduction"),
            params: &BAR,
        },
        build: |s, p| Box::new(Conducting::new(p.get(s, "hot"), p.get(s, "cold"), p.get(s, "first"), p.get(s, "second"), p.get(s, "length"), p.get(s, "area") * 1e-4)),
    },
    Scenario {
        spec: ScenarioSpec {
            id: "newtons-cooling",
            title: "Newton's law of cooling",
            description: "A hot cup cools quickly at first and then more and more slowly. It loses heat at a rate in proportion to how much hotter than the room it is, so its excess of temperature halves in equal times and never quite reaches the room's.",
            syllabus: syllabus(HEAT, "Newton's law of cooling"),
            params: &CUP,
        },
        build: |s, p| Box::new(Cooling::new(p.get(s, "start"), p.get(s, "room"), p.get(s, "rate"))),
    },
    Scenario {
        spec: ScenarioSpec {
            id: "blackbody-radiation",
            title: "Radiation: Wien's and Stefan's laws",
            description: "Anything hot glows. The hotter it is, the shorter the wavelength it is brightest at: red-hot, then white-hot, then blue. And the total it gives out rises as the fourth power of its temperature, so doubling the temperature gives sixteen times the power.",
            syllabus: syllabus(HEAT, "Heat transfer: radiation"),
            params: &GLOW,
        },
        build: |s, p| Box::new(Glowing { temperature: p.get(s, "temperature") }),
    },
];
