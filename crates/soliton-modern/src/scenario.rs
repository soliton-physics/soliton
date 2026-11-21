//! The catalogue: class 12 modern physics, chapter by chapter (design 017).

use crate::atom::{Atom, Series};
use crate::frame::{Answer, Lesson, Picture, Plot, Reading};
use crate::matter::{Diffraction, MatterWave};
use crate::nucleus::{Nuclide, Sample};
use crate::photoelectric::Tube;
use crate::rutherford::Foil;
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

const SEED: u64 = 2026;
const DUAL: &str = "Dual nature of radiation and matter";
const ATOMS: &str = "Atoms";
const NUCLEI: &str = "Nuclei";

const fn syllabus(chapter: &'static str, topic: &'static str) -> Syllabus {
    Syllabus {
        class: 12,
        chapter,
        topic,
    }
}

static TUBE: [Param; 4] = [
    param(
        "wavelength",
        "Wavelength of the light",
        "nm",
        (150.0, 750.0),
        5.0,
        400.0,
    ),
    param(
        "intensity",
        "Brightness of the light",
        "%",
        (0.0, 100.0),
        5.0,
        50.0,
    ),
    param(
        "voltage",
        "Voltage on the collector",
        "V",
        (-4.0, 4.0),
        0.05,
        0.0,
    ),
    param(
        "work_function",
        "Work function of the metal",
        "eV",
        (1.8, 6.0),
        0.01,
        2.14,
    ),
];

static WAVE: [Param; 1] = [param(
    "voltage",
    "Accelerating voltage",
    "V",
    (10.0, 1000.0),
    1.0,
    100.0,
)];

static CRYSTAL: [Param; 2] = [
    param(
        "voltage",
        "Accelerating voltage",
        "V",
        (35.0, 150.0),
        1.0,
        54.0,
    ),
    param(
        "spacing",
        "Distance between rows of atoms",
        "nm",
        (0.15, 0.35),
        0.005,
        0.215,
    ),
];

static FOIL: [Param; 3] = [
    param(
        "energy",
        "Energy of the alpha particles",
        "MeV",
        (3.0, 10.0),
        0.1,
        5.5,
    ),
    param(
        "protons",
        "Protons in the nucleus, Z",
        "",
        (13.0, 92.0),
        1.0,
        79.0,
    ),
    param(
        "impact",
        "Impact parameter of the marked alpha",
        "fm",
        (5.0, 300.0),
        1.0,
        30.0,
    ),
];

static ATOM: [Param; 3] = [
    param("upper", "Upper level, n₂", "", (2.0, 7.0), 1.0, 3.0),
    param("lower", "Lower level, n₁", "", (1.0, 5.0), 1.0, 2.0),
    param(
        "protons",
        "Charge of the nucleus, Z",
        "",
        (1.0, 3.0),
        1.0,
        1.0,
    ),
];

static SERIES: [Param; 2] = [
    param(
        "lower",
        "Series: the level the electron falls to",
        "",
        (1.0, 5.0),
        1.0,
        2.0,
    ),
    param("line", "Line of the series", "", (1.0, 8.0), 1.0, 1.0),
];

static SAMPLE: [Param; 2] = [
    param("half_life", "Half-life", "s", (1.0, 20.0), 0.5, 5.0),
    param(
        "nuclei",
        "Nuclei at the start",
        "",
        (100.0, 2500.0),
        100.0,
        900.0,
    ),
];

static NUCLIDE: [Param; 1] = [param(
    "mass_number",
    "Mass number, A",
    "",
    (16.0, 250.0),
    1.0,
    56.0,
)];

static CATALOGUE: &[Scenario] = &[
    Scenario {
        spec: ScenarioSpec {
            id: "photoelectric-effect",
            title: "The photoelectric effect",
            description: "Light falls on a metal plate and knocks electrons out of it, which a collector gathers. Make the light brighter and more electrons come, no faster. Make its wavelength shorter and they come faster. Past a threshold wavelength none come at all, however bright the light: each electron is freed by one photon, and the photon must have enough energy.",
            syllabus: syllabus(DUAL, "Einstein's photoelectric equation"),
            params: &TUBE,
        },
        build: |s, p| Box::new(Tube::new(p.get(s, "wavelength"), p.get(s, "intensity"), p.get(s, "voltage"), p.get(s, "work_function"), SEED)),
    },
    Scenario {
        spec: ScenarioSpec {
            id: "de-broglie-wavelength",
            title: "The wavelength of an electron",
            description: "De Broglie proposed that a moving particle has a wavelength, Planck's constant over its momentum. For an electron sped up through a hundred volts it comes to about the distance between atoms in a crystal, which is why a crystal can diffract electrons. For a cricket ball it is far too small ever to notice.",
            syllabus: syllabus(DUAL, "Wave nature of matter"),
            params: &WAVE,
        },
        build: |s, p| Box::new(MatterWave { voltage: p.get(s, "voltage"), spacing: 0.215 }),
    },
    Scenario {
        spec: ScenarioSpec {
            id: "davisson-germer",
            title: "The Davisson–Germer experiment",
            description: "Electrons are fired at a nickel crystal and counted as they come off at each angle. They do not scatter evenly: there is a hump at one angle, just where waves of de Broglie's wavelength from neighbouring rows of atoms arrive in step. At 54 volts the hump is at 50 degrees. Particles were diffracting like light.",
            syllabus: syllabus(DUAL, "Davisson and Germer experiment"),
            params: &CRYSTAL,
        },
        build: |s, p| Box::new(Diffraction { voltage: p.get(s, "voltage"), spacing: p.get(s, "spacing") }),
    },
    Scenario {
        spec: ScenarioSpec {
            id: "rutherford-scattering",
            title: "Rutherford's alpha scattering",
            description: "Alpha particles are fired past the nucleus of a gold atom. Most pass far from it and are hardly turned. The few that come close are swung through large angles, and one aimed straight at it comes back the way it came. Only a tiny, massive, charged centre can do that: the atom is mostly empty space.",
            syllabus: syllabus(ATOMS, "Alpha-particle scattering and the nuclear atom"),
            params: &FOIL,
        },
        build: |s, p| Box::new(Foil::new(p.get(s, "energy"), p.get(s, "protons"), p.get(s, "impact"))),
    },
    Scenario {
        spec: ScenarioSpec {
            id: "bohr-model",
            title: "Bohr's model of the hydrogen atom",
            description: "The electron may circle the nucleus only in certain orbits, each with a definite energy. In one of them it does not radiate. When it drops from a higher orbit to a lower one, the energy it loses leaves as a single photon, and that fixes the colour of the light.",
            syllabus: syllabus(ATOMS, "Bohr model and energy levels"),
            params: &ATOM,
        },
        build: |s, p| Box::new(Atom::new(p.get(s, "protons"), p.get(s, "upper") as u32, p.get(s, "lower") as u32)),
    },
    Scenario {
        spec: ScenarioSpec {
            id: "hydrogen-spectrum",
            title: "The line spectrum of hydrogen",
            description: "Glowing hydrogen gives out light of certain wavelengths only. They fall into series, one for each level the electron can end on: Lyman in the ultraviolet, Balmer in the visible, Paschen and the rest in the infrared. Within a series the lines crowd together towards a limit.",
            syllabus: syllabus(ATOMS, "Line spectra of the hydrogen atom"),
            params: &SERIES,
        },
        build: |s, p| Box::new(Series { lower: p.get(s, "lower") as u32, line: p.get(s, "line") as u32 }),
    },
    Scenario {
        spec: ScenarioSpec {
            id: "radioactive-decay",
            title: "Radioactive decay",
            description: "Every nucleus in the sample has the same chance of decaying in the next second, whatever its age. Nobody can say which will go next. Yet the number left falls along a smooth curve, halving in every half-life, because there are so many of them.",
            syllabus: syllabus(NUCLEI, "Law of radioactive decay"),
            params: &SAMPLE,
        },
        build: |s, p| Box::new(Sample::new(p.get(s, "nuclei") as usize, p.get(s, "half_life"), SEED)),
    },
    Scenario {
        spec: ScenarioSpec {
            id: "binding-energy",
            title: "The binding energy curve",
            description: "Binding energy per nucleon says how tightly a nucleus holds together. It rises steeply for light nuclei, peaks near iron, and falls slowly for heavy ones. So energy is released both when light nuclei fuse and when heavy ones split: each moves towards the peak.",
            syllabus: syllabus(NUCLEI, "Binding energy, fission and fusion"),
            params: &NUCLIDE,
        },
        build: |s, p| Box::new(Nuclide { a: p.get(s, "mass_number") as u32 }),
    },
];
