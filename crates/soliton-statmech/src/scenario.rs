//! The catalogue: first steps in computational statistical physics (design 016), beyond the school syllabus.

use crate::ising::{onsager_energy, onsager_magnetisation, CRITICAL_TEMPERATURE};
use crate::scene::{AnswerSpec, MeasureSpec, Model, StatScene, SyllabusSpec, SCHEMA};
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
            .map_or(param.default, |(_, v)| v.clamp(param.min, param.max))
    }
}

pub struct Scenario {
    pub spec: ScenarioSpec,
    describe: fn(&ScenarioSpec, &Params) -> StatScene,
}

impl Scenario {
    pub fn scene(&self, params: &Params) -> StatScene {
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

fn scene(spec: &ScenarioSpec, model: Model, playback: f64) -> StatScene {
    StatScene {
        schema: SCHEMA,
        title: spec.title.to_string(),
        description: None,
        syllabus: None,
        model,
        seed: 2026,
        playback,
        answers: Vec::new(),
    }
}

impl StatScene {
    fn answer(mut self, name: &str, value: f64) -> Self {
        self.answers.push(AnswerSpec {
            name: name.to_string(),
            value,
            measure: None,
            within: 0.0,
        });
        self
    }

    fn measured(mut self, name: &str, value: f64, measure: MeasureSpec, within: f64) -> Self {
        self.answers.push(AnswerSpec {
            name: name.to_string(),
            value,
            measure: Some(measure),
            within,
        });
        self
    }
}

static ISING: [Param; 3] = [
    param("temperature", "Temperature", "J/k", (0.5, 5.0), 0.05, 2.0),
    param("field", "Field", "J", (-1.0, 1.0), 0.05, 0.0),
    param("size", "Spins along a side", "", (16.0, 96.0), 8.0, 48.0),
];

fn ising_model(spec: &ScenarioSpec, p: &Params) -> StatScene {
    let (t, h) = (p.get(spec, "temperature"), p.get(spec, "field"));
    let size = p.get(spec, "size").round() as usize;
    let model = Model::Ising {
        size,
        temperature: t,
        field: h,
        // Begun in order below the critical temperature and in disorder above it, so that it has not far to
        // go to equilibrium.
        ordered: t < CRITICAL_TEMPERATURE,
        settle: 200,
    };
    let mut out = scene(spec, model, 30.0).answer(
        "Critical temperature, 2/ln(1 + √2) (J/k)",
        CRITICAL_TEMPERATURE,
    );
    // Onsager's results are for no field.
    if h == 0.0 {
        out = out
            .measured(
                "Magnetisation per spin, Onsager's formula",
                onsager_magnetisation(t),
                MeasureSpec::Magnetisation,
                // A finite lattice loses its magnetisation gradually where an infinite one loses it at once, and
                // above the critical temperature it keeps a little, of the order of one over its side.
                if (t - CRITICAL_TEMPERATURE).abs() < 0.35 {
                    0.25
                } else if t > CRITICAL_TEMPERATURE {
                    0.12
                } else {
                    0.04
                },
            )
            .measured(
                "Energy per spin, Onsager's formula (J)",
                onsager_energy(t),
                MeasureSpec::Energy,
                0.06,
            );
    }
    out
}

static TRANSITION: [Param; 1] = [param(
    "size",
    "Spins along a side",
    "",
    (16.0, 64.0),
    8.0,
    32.0,
)];

fn phase_transition(spec: &ScenarioSpec, p: &Params) -> StatScene {
    let size = p.get(spec, "size").round() as usize;
    let model = Model::Sweep {
        size,
        from: 3.6,
        to: 1.2,
        points: 25,
        settle: 200,
        measure: 400,
    };
    scene(spec, model, 400.0)
        .answer(
            "Critical temperature, 2/ln(1 + √2) (J/k)",
            CRITICAL_TEMPERATURE,
        )
        .measured(
            "Temperature at which the specific heat peaks (J/k)",
            CRITICAL_TEMPERATURE,
            MeasureSpec::HeatPeak,
            0.2,
        )
        .answer("Magnetisation well below it", 1.0)
        .answer("Magnetisation well above it", 0.0)
}

static DARTS: [Param; 1] = [param(
    "batch",
    "Points thrown at each step",
    "",
    (1.0, 2000.0),
    1.0,
    50.0,
)];

fn monte_carlo_pi(spec: &ScenarioSpec, p: &Params) -> StatScene {
    let batch = p.get(spec, "batch").round() as usize;
    scene(spec, Model::Darts { batch }, 30.0).measured(
        "π",
        std::f64::consts::PI,
        MeasureSpec::Estimate,
        0.02,
    )
}

static WALK: [Param; 2] = [
    param(
        "walkers",
        "Number of walkers",
        "",
        (100.0, 5000.0),
        100.0,
        2000.0,
    ),
    param(
        "steps",
        "Steps each takes",
        "",
        (100.0, 2500.0),
        100.0,
        900.0,
    ),
];

fn random_walk(spec: &ScenarioSpec, p: &Params) -> StatScene {
    let walkers = p.get(spec, "walkers").round() as usize;
    let steps = p.get(spec, "steps").round();
    let far = steps.sqrt();
    scene(
        spec,
        Model::Walk {
            walkers,
            steps: steps as u64,
        },
        60.0,
    )
    .measured(
        "Root mean square distance at the end, √N (steps)",
        far,
        MeasureSpec::RmsDistance,
        // The spread of the estimate from this many walkers, three times over.
        3.0 * far / (2.0 * (walkers as f64).sqrt()),
    )
    .measured(
        "How far the middle of the crowd has moved (steps)",
        0.0,
        MeasureSpec::Drift,
        3.0 * far / (walkers as f64).sqrt(),
    )
    .answer("Distance if every step were the same way (steps)", steps)
}

const MONTE_CARLO: &str = "Monte Carlo methods";
const TRANSITIONS: &str = "Phase transitions";

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
        // Not a school class: university statistical physics.
        syllabus: Syllabus {
            class: 0,
            chapter,
            topic,
        },
        params,
    }
}

#[rustfmt::skip]
static CATALOGUE: &[Scenario] = &[
    Scenario {
        spec: spec("monte-carlo-pi", "Finding π by throwing darts", "Throw points at a square at random and count how many land inside the quarter circle. The fraction is the ratio of the two areas, π/4. The answer is rough at first and sharpens slowly: a hundred times as many points for one more digit.", MONTE_CARLO, "Monte Carlo integration", &DARTS),
        describe: monte_carlo_pi,
    },
    Scenario {
        spec: spec("random-walk", "A crowd of random walkers", "Each walker takes a step north, south, east or west at random, again and again. No one of them gets anywhere in particular, yet the crowd spreads in a perfectly regular way: its width grows as the square root of the number of steps. This is diffusion.", MONTE_CARLO, "random walks and diffusion", &WALK),
        describe: random_walk,
    },
    Scenario {
        spec: spec("ising-model", "The Ising model of a magnet", "Each square is an atom's spin, up or down. Neighbours prefer to agree, and heat jostles them out of agreement. Below a critical temperature agreement wins and the whole lattice is magnetised; above it the spins are a flickering patchwork. Slide the temperature across 2.27 and watch it change character.", TRANSITIONS, "the Ising model and the Metropolis algorithm", &ISING),
        describe: ising_model,
    },
    Scenario {
        spec: spec("phase-transition", "Cooling a magnet through its critical point", "The lattice is cooled in steps from well above the critical temperature to well below, and measured at each. The magnetisation appears suddenly and the specific heat rises to a peak at one temperature: a phase transition, found from nothing but spins flipping at random.", TRANSITIONS, "critical phenomena", &TRANSITION),
        describe: phase_transition,
    },
];
