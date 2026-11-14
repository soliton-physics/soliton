//! The catalogue: NCERT class 11, thermodynamics and kinetic theory, and the first steps of statistical
//! mechanics beyond it (design 014). Each scenario is a function from a few parameters to a scene, with the
//! textbook's answer beside what the molecules give.

use crate::scene::{
    AnswerSpec, GroupSpec, MeasureSpec, StageSpec, SyllabusSpec, ThermalScene, WallsSpec, SCHEMA,
};
use crate::{ATMOSPHERE, BOLTZMANN, JOULES, METRES_PER_SECOND, PASCALS};
use serde::Serialize;
use std::f64::consts::PI;

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
            .map_or(param.default, |(_, v)| v.clamp(param.min, param.max))
    }
}

pub struct Scenario {
    pub spec: ScenarioSpec,
    describe: fn(&ScenarioSpec, &Params) -> ThermalScene,
}

impl Scenario {
    pub fn scene(&self, params: &Params) -> ThermalScene {
        let spec = &self.spec;
        let mut scene = (self.describe)(spec, params);
        scene.description = Some(spec.description.to_string());
        scene.syllabus = Some(SyllabusSpec {
            class: spec.syllabus.class,
            chapter: spec.syllabus.chapter.to_string(),
            topic: spec.syllabus.topic.to_string(),
        });
        scene
    }
}

pub fn catalogue() -> &'static [Scenario] {
    CATALOGUE
}

pub fn find(id: &str) -> Option<&'static Scenario> {
    CATALOGUE.iter().find(|s| s.spec.id == id)
}

/// Argon: one atom, so nothing to it but its motion from place to place. Hard spheres cannot spin or stretch,
/// which makes every gas here monatomic, with γ = 5/3.
const ARGON: f64 = 39.95;
const ARGON_RADIUS: f64 = 0.15;
const NEON: f64 = 20.18;
const XENON: f64 = 131.3;
const DT: f64 = 0.05;
/// Energies are quoted in units of 10⁻²¹ J, the size of kT at room temperature.
const ZEPTO: f64 = JOULES * 1e21;

fn atmospheres(pressure: f64) -> f64 {
    pressure * PASCALS / ATMOSPHERE
}

fn group(name: &str, count: f64, mass: f64, radius: f64, temperature: f64) -> GroupSpec {
    GroupSpec {
        name: name.to_string(),
        count: count.round() as usize,
        mass,
        radius,
        temperature,
        region: None,
        equal_speeds: false,
    }
}

fn scene(spec: &ScenarioSpec, size: [f64; 3], playback: f64) -> ThermalScene {
    ThermalScene {
        schema: SCHEMA,
        title: spec.title.to_string(),
        description: None,
        syllabus: None,
        size,
        groups: Vec::new(),
        partition: None,
        collisions: true,
        dt: DT,
        playback,
        seed: 2026,
        stages: Vec::new(),
        answers: Vec::new(),
    }
}

impl ThermalScene {
    fn with(mut self, group: GroupSpec) -> Self {
        self.groups.push(group);
        self
    }

    fn stage(mut self, stage: StageSpec) -> Self {
        self.stages.push(stage);
        self
    }

    fn answer(mut self, name: &str, value: f64) -> Self {
        self.answers.push(AnswerSpec {
            name: name.to_string(),
            value,
            measure: None,
            stage: None,
            tolerance: 0.03,
        });
        self
    }

    /// An answer the molecules can be asked for, during `stage`, to within `tolerance`.
    fn measured(
        mut self,
        name: &str,
        value: f64,
        measure: MeasureSpec,
        stage: usize,
        tolerance: f64,
    ) -> Self {
        self.answers.push(AnswerSpec {
            name: name.to_string(),
            value,
            measure: Some(measure),
            stage: Some(stage),
            tolerance,
        });
        self
    }
}

fn rms(mass: f64, temperature: f64) -> f64 {
    (3.0 * BOLTZMANN * temperature / mass).sqrt() * METRES_PER_SECOND
}

fn mean(mass: f64, temperature: f64) -> f64 {
    (8.0 * BOLTZMANN * temperature / (PI * mass)).sqrt() * METRES_PER_SECOND
}

// ---------------------------------------------------------------- kinetic theory

static IDEAL_GAS: [Param; 3] = [
    param(
        "count",
        "Number of molecules",
        "",
        (50.0, 600.0),
        10.0,
        300.0,
    ),
    param(
        "temperature",
        "Temperature",
        "K",
        (100.0, 1000.0),
        10.0,
        300.0,
    ),
    param("side", "Side of the box", "nm", (8.0, 20.0), 0.5, 12.0),
];

fn ideal_gas(spec: &ScenarioSpec, p: &Params) -> ThermalScene {
    let (n, t, side) = (
        p.get(spec, "count").round(),
        p.get(spec, "temperature"),
        p.get(spec, "side"),
    );
    let volume = side.powi(3);
    scene(spec, [side; 3], 20.0)
        .with(group("argon", n, ARGON, ARGON_RADIUS, t))
        .stage(StageSpec::new("In equilibrium", WallsSpec::Insulated))
        .measured(
            "Pressure, NkT/V (atm)",
            atmospheres(n * BOLTZMANN * t / volume),
            MeasureSpec::Pressure,
            0,
            0.04,
        )
        .measured(
            "Root mean square speed, √(3kT/m) (m/s)",
            rms(ARGON, t),
            MeasureSpec::RmsSpeed { species: 0 },
            0,
            0.01,
        )
        .measured(
            "Mean kinetic energy of a molecule, 3kT/2 (×10⁻²¹ J)",
            1.5 * BOLTZMANN * t * ZEPTO,
            MeasureSpec::EnergyPerMolecule,
            0,
            0.01,
        )
        .answer(
            "Internal energy, 3NkT/2 (×10⁻²¹ J)",
            1.5 * n * BOLTZMANN * t * ZEPTO,
        )
}

static MAXWELL: [Param; 2] = [
    param(
        "temperature",
        "Temperature",
        "K",
        (100.0, 1000.0),
        10.0,
        300.0,
    ),
    param("mass", "Mass of a molecule", "u", (4.0, 131.0), 1.0, 40.0),
];

fn maxwell_distribution(spec: &ScenarioSpec, p: &Params) -> ThermalScene {
    let (t, mass) = (p.get(spec, "temperature"), p.get(spec, "mass"));
    let mut atoms = group("gas", 400.0, mass, 0.17, t);
    atoms.equal_speeds = true;
    // Long enough for every molecule to have collided several times.
    let settling = 12.0 * (mass / 40.0 * 300.0 / t).sqrt() * 12.0;
    scene(spec, [10.0; 3], settling / 8.0)
        .with(atoms)
        .stage(StageSpec::new("Coming to equilibrium", WallsSpec::Insulated).lasting(settling))
        .stage(StageSpec::new("In equilibrium", WallsSpec::Insulated))
        .answer(
            "Most probable speed, √(2kT/m) (m/s)",
            (2.0 * BOLTZMANN * t / mass).sqrt() * METRES_PER_SECOND,
        )
        .measured(
            "Mean speed, √(8kT/πm) (m/s)",
            mean(mass, t),
            MeasureSpec::MeanSpeed { species: 0 },
            1,
            0.02,
        )
        .measured(
            "Root mean square speed, √(3kT/m) (m/s)",
            rms(mass, t),
            MeasureSpec::RmsSpeed { species: 0 },
            1,
            0.01,
        )
}

static MIXTURE: [Param; 3] = [
    param("light", "Neon starts at", "K", (100.0, 900.0), 10.0, 600.0),
    param("heavy", "Xenon starts at", "K", (100.0, 900.0), 10.0, 200.0),
    param("count", "Molecules of each", "", (50.0, 250.0), 10.0, 150.0),
];

fn gas_mixture(spec: &ScenarioSpec, p: &Params) -> ThermalScene {
    let (light, heavy, n) = (
        p.get(spec, "light"),
        p.get(spec, "heavy"),
        p.get(spec, "count").round(),
    );
    // Equal numbers, and every molecule holds 3kT/2 whatever its mass: they meet halfway.
    let shared = 0.5 * (light + heavy);
    scene(spec, [10.0; 3], 40.0)
        .with(group("neon", n, NEON, 0.14, light))
        .with(group("xenon", n, XENON, 0.2, heavy))
        .stage(StageSpec::new("Sharing their energy", WallsSpec::Insulated).lasting(600.0))
        .stage(StageSpec::new("In equilibrium", WallsSpec::Insulated))
        .measured(
            "Temperature they settle at (K)",
            shared,
            MeasureSpec::Temperature,
            1,
            0.001,
        )
        .measured(
            "Temperature of the neon alone (K)",
            shared,
            MeasureSpec::SpeciesTemperature { species: 0 },
            1,
            0.06,
        )
        .measured(
            "Temperature of the xenon alone (K)",
            shared,
            MeasureSpec::SpeciesTemperature { species: 1 },
            1,
            0.06,
        )
        .measured(
            "Root mean square speed of neon (m/s)",
            rms(NEON, shared),
            MeasureSpec::RmsSpeed { species: 0 },
            1,
            0.04,
        )
        .measured(
            "Root mean square speed of xenon (m/s)",
            rms(XENON, shared),
            MeasureSpec::RmsSpeed { species: 1 },
            1,
            0.04,
        )
}

static BROWNIAN: [Param; 2] = [
    param(
        "temperature",
        "Temperature",
        "K",
        (100.0, 1000.0),
        10.0,
        300.0,
    ),
    param(
        "mass",
        "Mass of the grain",
        "u",
        (500.0, 5000.0),
        100.0,
        2000.0,
    ),
];

fn brownian_motion(spec: &ScenarioSpec, p: &Params) -> ThermalScene {
    let (t, mass) = (p.get(spec, "temperature"), p.get(spec, "mass"));
    scene(spec, [12.0; 3], 40.0)
        // The grain goes in first, while there is room for it, and starts at rest.
        .with(group("grain", 1.0, mass, 0.9, 0.0))
        .with(group("argon", 500.0, ARGON, ARGON_RADIUS, t))
        .stage(StageSpec::new("Being knocked about", WallsSpec::Bath(t)))
        .measured(
            "Temperature of the grain, averaged (K)",
            t,
            MeasureSpec::SpeciesTemperature { species: 0 },
            0,
            0.25,
        )
        .measured(
            "Root mean square speed of the grain, √(3kT/M) (m/s)",
            rms(mass, t),
            MeasureSpec::RmsSpeed { species: 0 },
            0,
            0.15,
        )
        .answer(
            "Root mean square speed of an argon atom (m/s)",
            rms(ARGON, t),
        )
}

// ---------------------------------------------------------------- thermodynamics

const CYLINDER: f64 = 10.0;
/// A hundredth of the speed of the molecules. A piston that moves faster meets them harder than a slow one would
/// and does more work than the reversible formulas allow for.
const PISTON_SPEED: f64 = 0.004;

static ISOTHERMAL: [Param; 3] = [
    param(
        "temperature",
        "Temperature of the walls",
        "K",
        (150.0, 800.0),
        10.0,
        300.0,
    ),
    param(
        "from",
        "Length to start with",
        "nm",
        (10.0, 20.0),
        0.5,
        16.0,
    ),
    param("to", "Length to end with", "nm", (5.0, 24.0), 0.5, 8.0),
];

static ADIABATIC: [Param; 3] = [
    param(
        "temperature",
        "Temperature to start with",
        "K",
        (150.0, 800.0),
        10.0,
        300.0,
    ),
    param(
        "from",
        "Length to start with",
        "nm",
        (10.0, 20.0),
        0.5,
        16.0,
    ),
    param("to", "Length to end with", "nm", (5.0, 24.0), 0.5, 8.0),
];

const IN_CYLINDER: f64 = 250.0;

fn isothermal(spec: &ScenarioSpec, p: &Params) -> ThermalScene {
    let (t, from, to) = (
        p.get(spec, "temperature"),
        p.get(spec, "from"),
        p.get(spec, "to"),
    );
    let n = IN_CYLINDER;
    let area = CYLINDER * CYLINDER;
    let work = n * BOLTZMANN * t * (from / to).ln();
    scene(spec, [from, CYLINDER, CYLINDER], 100.0)
        .with(group("argon", n, ARGON, ARGON_RADIUS, t))
        .stage(StageSpec::new("Before", WallsSpec::Bath(t)).lasting(60.0))
        .stage(
            StageSpec::new(
                "Piston moving, walls at a fixed temperature",
                WallsSpec::Bath(t),
            )
            .piston(to, PISTON_SPEED),
        )
        .stage(StageSpec::new("After", WallsSpec::Bath(t)))
        .measured(
            "Pressure before (atm)",
            atmospheres(n * BOLTZMANN * t / (area * from)),
            MeasureSpec::Pressure,
            0,
            0.06,
        )
        .measured(
            "Pressure after, P₁V₁/V₂ (atm)",
            atmospheres(n * BOLTZMANN * t / (area * to)),
            MeasureSpec::Pressure,
            2,
            0.05,
        )
        .measured(
            "Work done on the gas, NkT ln(V₁/V₂) (×10⁻²¹ J)",
            work * ZEPTO,
            MeasureSpec::Work,
            1,
            0.06,
        )
        // The gas's own energy flickers by a few per cent while it touches the walls, and whatever it happens
        // to hold at each end of the stroke shows up in the heat.
        .measured(
            "Heat given to the gas (×10⁻²¹ J)",
            -work * ZEPTO,
            MeasureSpec::Heat,
            1,
            0.4,
        )
        .measured(
            "Temperature after, averaged (K)",
            t,
            MeasureSpec::MeanTemperature,
            2,
            0.03,
        )
}

fn adiabatic(spec: &ScenarioSpec, p: &Params) -> ThermalScene {
    let (t, from, to) = (
        p.get(spec, "temperature"),
        p.get(spec, "from"),
        p.get(spec, "to"),
    );
    let n = IN_CYLINDER;
    let area = CYLINDER * CYLINDER;
    // T V^(γ−1) stays the same, with γ = 5/3.
    let after = t * (from / to).powf(2.0 / 3.0);
    let work = 1.5 * n * BOLTZMANN * (after - t);
    scene(spec, [from, CYLINDER, CYLINDER], 100.0)
        .with(group("argon", n, ARGON, ARGON_RADIUS, t))
        .stage(StageSpec::new("Before", WallsSpec::Insulated).lasting(60.0))
        .stage(
            StageSpec::new("Piston moving, walls insulated", WallsSpec::Insulated)
                .piston(to, PISTON_SPEED),
        )
        .stage(StageSpec::new("After", WallsSpec::Insulated))
        .measured(
            "Temperature after, T₁(V₁/V₂)^(γ−1) (K)",
            after,
            MeasureSpec::Temperature,
            2,
            0.03,
        )
        .measured(
            "Pressure after, P₁(V₁/V₂)^γ (atm)",
            atmospheres(n * BOLTZMANN * after / (area * to)),
            MeasureSpec::Pressure,
            2,
            0.05,
        )
        .measured(
            "Work done on the gas, 3Nk(T₂ − T₁)/2 (×10⁻²¹ J)",
            work * ZEPTO,
            MeasureSpec::Work,
            1,
            0.07,
        )
        .measured(
            "Heat given to the gas (×10⁻²¹ J)",
            0.0,
            MeasureSpec::Heat,
            1,
            0.03,
        )
}

static FREE_EXPANSION: [Param; 2] = [
    param(
        "temperature",
        "Temperature",
        "K",
        (100.0, 1000.0),
        10.0,
        300.0,
    ),
    param(
        "count",
        "Number of molecules",
        "",
        (100.0, 500.0),
        10.0,
        300.0,
    ),
];

fn free_expansion(spec: &ScenarioSpec, p: &Params) -> ThermalScene {
    let (t, n) = (p.get(spec, "temperature"), p.get(spec, "count").round());
    let (half, full) = (8.0, 16.0);
    let area = CYLINDER * CYLINDER;
    scene(spec, [half, CYLINDER, CYLINDER], 20.0)
        .with(group("argon", n, ARGON, ARGON_RADIUS, t))
        .stage(StageSpec::new("Held in half the box", WallsSpec::Insulated).lasting(60.0))
        // The wall is gone before any molecule can push on it.
        .stage(
            StageSpec::new("The wall is taken away", WallsSpec::Insulated)
                .piston(full, 0.0)
                .lasting(120.0),
        )
        .stage(StageSpec::new(
            "Spread through the whole box",
            WallsSpec::Insulated,
        ))
        .measured(
            "Temperature after (K)",
            t,
            MeasureSpec::Temperature,
            2,
            0.001,
        )
        .measured(
            "Pressure before (atm)",
            atmospheres(n * BOLTZMANN * t / (area * half)),
            MeasureSpec::Pressure,
            0,
            0.06,
        )
        .measured(
            "Pressure after: half of that (atm)",
            atmospheres(n * BOLTZMANN * t / (area * full)),
            MeasureSpec::Pressure,
            2,
            0.06,
        )
        .measured(
            "Work done on the gas (×10⁻²¹ J)",
            0.0,
            MeasureSpec::Work,
            1,
            0.01,
        )
        .answer(
            "Entropy gained, Nk ln 2 (×10⁻²¹ J/K)",
            n * BOLTZMANN * 2f64.ln() * ZEPTO,
        )
}

static MIXING: [Param; 2] = [
    param(
        "temperature",
        "Temperature",
        "K",
        (100.0, 1000.0),
        10.0,
        300.0,
    ),
    param(
        "count",
        "Molecules of each gas",
        "",
        (50.0, 250.0),
        10.0,
        150.0,
    ),
];

fn mixing(spec: &ScenarioSpec, p: &Params) -> ThermalScene {
    let (t, n) = (p.get(spec, "temperature"), p.get(spec, "count").round());
    let length = 16.0;
    let mut left = group("neon", n, NEON, 0.14, t);
    left.region = Some([0.0, 0.5]);
    let mut right = group("argon", n, ARGON, ARGON_RADIUS, t);
    right.region = Some([0.5, 1.0]);
    let mut out = scene(spec, [length, CYLINDER, CYLINDER], 20.0)
        .with(left)
        .with(right)
        .stage(StageSpec::new("Kept apart", WallsSpec::Insulated).lasting(40.0))
        .stage(
            StageSpec::new("The wall is taken away", WallsSpec::Insulated)
                .without_partition()
                .lasting(400.0),
        )
        .stage(StageSpec::new("Mixed", WallsSpec::Insulated))
        .measured(
            "Share of the neon in the left half, once mixed",
            0.5,
            MeasureSpec::ShareOnLeft { species: 0 },
            2,
            0.08,
        )
        .measured(
            "Share of the argon in the left half, once mixed",
            0.5,
            MeasureSpec::ShareOnLeft { species: 1 },
            2,
            0.08,
        )
        .measured(
            "Temperature after (K)",
            t,
            MeasureSpec::Temperature,
            2,
            0.001,
        )
        .answer(
            "Entropy gained, 2Nk ln 2 (×10⁻²¹ J/K)",
            2.0 * n * BOLTZMANN * 2f64.ln() * ZEPTO,
        );
    out.partition = Some(0.5 * length);
    out
}

static CARNOT: [Param; 3] = [
    param("hot", "Hot reservoir", "K", (350.0, 700.0), 10.0, 450.0),
    param("cold", "Cold reservoir", "K", (150.0, 330.0), 10.0, 300.0),
    param(
        "ratio",
        "Expansion while taking in heat",
        "×",
        (1.2, 2.0),
        0.05,
        1.5,
    ),
];

fn carnot(spec: &ScenarioSpec, p: &Params) -> ThermalScene {
    let (hot, cold, ratio) = (
        p.get(spec, "hot"),
        p.get(spec, "cold"),
        p.get(spec, "ratio"),
    );
    let n = 400.0;
    let start = 8.0;
    // Along an adiabat T V^(2/3) is fixed, so cooling from hot to cold stretches the gas by this factor.
    let stretch = (hot / cold).powf(1.5);
    let lengths = [
        start,
        start * ratio,
        start * ratio * stretch,
        start * stretch,
    ];
    let taken_in = n * BOLTZMANN * hot * ratio.ln();
    let given_out = n * BOLTZMANN * cold * ratio.ln();
    let speed = 0.003;
    let playback = 300.0;
    let mut out = scene(spec, [start, CYLINDER, CYLINDER], playback);
    // Thousands of picoseconds of slow strokes: a longer step, still a small part of an atom's width.
    out.dt = 0.1;
    out.with(group("argon", n, ARGON, ARGON_RADIUS, hot))
        .stage(StageSpec::new("Ready, at the hot reservoir", WallsSpec::Bath(hot)).lasting(60.0))
        .stage(
            StageSpec::new(
                "1. Expanding at the hot reservoir: heat in",
                WallsSpec::Bath(hot),
            )
            .piston(lengths[1], speed),
        )
        .stage(
            StageSpec::new("2. Expanding, insulated: cooling", WallsSpec::Insulated)
                .piston(lengths[2], speed),
        )
        .stage(
            StageSpec::new(
                "3. Compressed at the cold reservoir: heat out",
                WallsSpec::Bath(cold),
            )
            .piston(lengths[3], speed),
        )
        .stage(
            StageSpec::new("4. Compressed, insulated: warming", WallsSpec::Insulated)
                .piston(lengths[0], speed),
        )
        .measured(
            "Efficiency, 1 − T꜀/Tₕ",
            1.0 - cold / hot,
            MeasureSpec::Efficiency { hot: 1, cold: 3 },
            4,
            0.1,
        )
        .measured(
            "Work done on the gas at the hot reservoir (×10⁻²¹ J)",
            -taken_in * ZEPTO,
            MeasureSpec::Work,
            1,
            0.06,
        )
        .measured(
            "Work done on the gas at the cold reservoir (×10⁻²¹ J)",
            given_out * ZEPTO,
            MeasureSpec::Work,
            3,
            0.06,
        )
        .answer(
            "Heat taken in at the hot reservoir (×10⁻²¹ J)",
            taken_in * ZEPTO,
        )
        .answer(
            "Heat given out at the cold reservoir (×10⁻²¹ J)",
            given_out * ZEPTO,
        )
        .answer(
            "Work done by the gas in one cycle (×10⁻²¹ J)",
            (taken_in - given_out) * ZEPTO,
        )
}

// ---------------------------------------------------------------- catalogue

const KINETIC: &str = "Kinetic theory";
const THERMO: &str = "Thermodynamics";

const fn spec(
    id: &'static str,
    title: &'static str,
    description: &'static str,
    chapter: &'static str,
    topic: &'static str,
    params: &'static [Param],
) -> ScenarioSpec {
    ScenarioSpec {
        id,
        title,
        description,
        syllabus: Syllabus {
            class: 11,
            chapter,
            topic,
        },
        params,
    }
}

#[rustfmt::skip]
static CATALOGUE: &[Scenario] = &[
    Scenario {
        spec: spec("ideal-gas", "The pressure of a gas", "A gas is mostly empty space with molecules flying through it. Its pressure is nothing but their blows on the walls, counted here one by one. More molecules, a higher temperature or a smaller box, and the count goes up exactly as PV = NkT says.", KINETIC, "pressure of an ideal gas", &IDEAL_GAS),
        describe: ideal_gas,
    },
    Scenario {
        spec: spec("maxwell-distribution", "The Maxwell distribution of speeds", "Every molecule starts at the same speed. A few collisions later some are almost at rest and some are racing, and the spread settles into one particular shape that never changes again: Maxwell's. Nobody arranges it. It is simply the most likely way to share the energy.", KINETIC, "distribution of molecular speeds", &MAXWELL),
        describe: maxwell_distribution,
    },
    Scenario {
        spec: spec("gas-mixture", "Light and heavy molecules together", "Hot neon and cold xenon in one box. Collision by collision they share their energy until a neon atom and a xenon atom have, on average, the same kinetic energy. The same energy, not the same speed: the heavy ones move much more slowly.", KINETIC, "law of equipartition of energy", &MIXTURE),
        describe: gas_mixture,
    },
    Scenario {
        spec: spec("brownian-motion", "Brownian motion", "A grain fifty times heavier than the atoms round it, and far too big to be pushed about by any one of them. Yet it never rests: the blows on its two sides never quite balance. Its jittery wandering is how molecules were first shown to be real.", KINETIC, "Brownian motion", &BROWNIAN),
        describe: brownian_motion,
    },
    Scenario {
        spec: spec("isothermal-process", "Compressing a gas slowly, at a fixed temperature", "The piston squeezes the gas while the walls are kept at one temperature. Every molecule the piston speeds up soon hits a wall and gives the extra energy away. So the gas ends no hotter: all the work done on it has left as heat.", THERMO, "isothermal process", &ISOTHERMAL),
        describe: isothermal,
    },
    Scenario {
        spec: spec("adiabatic-process", "Compressing a gas with no heat allowed out", "The same piston, but the walls are insulated. A molecule that bounces off an advancing piston comes away faster, and now that energy has nowhere to go. The gas heats up, and its pressure rises more steeply than at a fixed temperature.", THERMO, "adiabatic process", &ADIABATIC),
        describe: adiabatic,
    },
    Scenario {
        spec: spec("free-expansion", "Free expansion", "A gas held in half a box, then the wall is whipped away. It rushes to fill the whole box. It pushed on nothing, so it did no work and is exactly as hot as before. And it will never, by itself, go back into the half it came from.", THERMO, "irreversible processes", &FREE_EXPANSION),
        describe: free_expansion,
    },
    Scenario {
        spec: spec("mixing", "Two gases mixing", "Neon on the left, argon on the right, at the same temperature and pressure. Take the wall away and nothing pushes them together, yet they mix completely, and they will not unmix. Nothing in the laws of motion forbids it; there are just overwhelmingly more ways to be mixed.", THERMO, "irreversible processes", &MIXING),
        describe: mixing,
    },
    Scenario {
        spec: spec("carnot-cycle", "The Carnot cycle", "The most efficient engine there can be between two temperatures, built from four slow strokes: expand while taking heat from the hot reservoir, expand insulated until cold, compress while giving heat to the cold reservoir, compress insulated until hot again. What it turns into work depends only on the two temperatures.", THERMO, "Carnot engine", &CARNOT),
        describe: carnot,
    },
];
