//! A statistical experiment as data: which model, how it is run, and the answers with how each is read off.

use crate::ising::Ising;
use crate::sampling::{Darts, Walkers};
use serde::{Deserialize, Serialize};

pub const SCHEMA: u32 = 1;

#[derive(Debug, Clone, PartialEq)]
pub struct SceneError(pub String);

impl std::fmt::Display for SceneError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for SceneError {}

fn err<T>(message: impl Into<String>) -> Result<T, SceneError> {
    Err(SceneError(message.into()))
}

fn is_false(v: &bool) -> bool {
    !*v
}

/// What is simulated. One step of an experiment is one sweep of a lattice, one batch of darts, or one step of
/// every walker.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Model {
    /// A lattice of spins held at one temperature. Averages are taken after `settle` sweeps.
    Ising {
        size: usize,
        temperature: f64,
        field: f64,
        /// Start with every spin up; otherwise at random, as if from a very high temperature.
        #[serde(default, skip_serializing_if = "is_false")]
        ordered: bool,
        settle: u64,
    },
    /// A lattice taken through `points` temperatures from `from` to `to`. At each it is given `settle` sweeps
    /// to come to equilibrium and then measured over `measure` more.
    Sweep {
        size: usize,
        from: f64,
        to: f64,
        points: usize,
        settle: u64,
        measure: u64,
    },
    /// Points thrown at a square, `batch` at a step, to find the area of a quarter circle.
    Darts { batch: usize },
    /// Walkers on a grid, for `steps` steps.
    Walk { walkers: usize, steps: u64 },
}

/// How an answer is read off the run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeasureSpec {
    /// Size of the magnetisation per spin, averaged since the lattice settled.
    Magnetisation,
    /// Energy per spin, averaged since the lattice settled.
    Energy,
    /// The temperature at which the specific heat was largest, once a sweep has finished.
    HeatPeak,
    /// Four times the fraction of darts inside the quarter circle.
    Estimate,
    /// Root mean square distance of the walkers from the start.
    RmsDistance,
    /// How far the middle of the crowd of walkers is from the start.
    Drift,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnswerSpec {
    pub name: String,
    pub value: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub measure: Option<MeasureSpec>,
    /// How close the run has to come to count as agreeing, in the answer's own units.
    #[serde(default)]
    pub within: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SyllabusSpec {
    pub class: u8,
    pub chapter: String,
    pub topic: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatScene {
    pub schema: u32,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub syllabus: Option<SyllabusSpec>,
    pub model: Model,
    pub seed: u64,
    /// Steps to make in one second of watching.
    pub playback: f64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub answers: Vec<AnswerSpec>,
}

/// Running sums over the sweeps of a lattice, from which its averages and their spreads come.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Averages {
    pub count: f64,
    magnetisation: f64,
    magnetisation_squared: f64,
    energy: f64,
    energy_squared: f64,
}

impl Averages {
    fn add(&mut self, lattice: &Ising) {
        let (m, e) = (lattice.magnetisation().abs(), lattice.energy());
        self.count += 1.0;
        self.magnetisation += m;
        self.magnetisation_squared += m * m;
        self.energy += e;
        self.energy_squared += e * e;
    }

    pub fn magnetisation(&self) -> f64 {
        self.magnetisation / self.count.max(1.0)
    }

    pub fn energy(&self) -> f64 {
        self.energy / self.count.max(1.0)
    }

    /// Specific heat per spin, in units of `k`, from how much the energy fluctuates: `N(⟨e²⟩ − ⟨e⟩²)/T²`.
    pub fn specific_heat(&self, spins: f64, temperature: f64) -> f64 {
        let spread = self.energy_squared / self.count.max(1.0) - self.energy().powi(2);
        spins * spread.max(0.0) / (temperature * temperature)
    }

    /// Susceptibility per spin from how much the magnetisation fluctuates: `N(⟨m²⟩ − ⟨|m|⟩²)/T`.
    pub fn susceptibility(&self, spins: f64, temperature: f64) -> f64 {
        let spread =
            self.magnetisation_squared / self.count.max(1.0) - self.magnetisation().powi(2);
        spins * spread.max(0.0) / temperature
    }
}

/// One temperature of a sweep, measured.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub temperature: f64,
    pub magnetisation: f64,
    pub energy: f64,
    pub specific_heat: f64,
    pub susceptibility: f64,
}

/// What is being run, with what it has accumulated.
#[derive(Clone, Debug)]
pub enum State {
    Lattice {
        lattice: Ising,
        averages: Averages,
        /// For a sweep: the temperatures measured so far, and how many sweeps the present one has had.
        points: Vec<Point>,
        spent: u64,
    },
    Darts(Darts),
    Walk(Walkers),
}

/// A scene being run.
#[derive(Clone, Debug)]
pub struct Run {
    pub scene: StatScene,
    pub state: State,
    pub steps: u64,
    /// A thinned record for graphs: `[step, first quantity, second quantity]`. Magnetisation and energy for a
    /// lattice, the estimate and its expected error for darts, the walkers' distance and `√steps` for a walk.
    pub history: Vec<[f64; 3]>,
}

/// How many dots of the darts are kept for drawing.
const DARTS_SHOWN: usize = 4000;

impl StatScene {
    pub fn from_json(text: &str) -> Result<Self, SceneError> {
        let scene: Self = serde_json::from_str(text)
            .map_err(|e| SceneError(format!("invalid scene JSON: {e}")))?;
        scene.start()?;
        Ok(scene)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("a scene always serialises")
    }

    pub fn start(&self) -> Result<Run, SceneError> {
        if self.schema != SCHEMA {
            return err(format!("unsupported scene schema {}", self.schema));
        }
        if self.playback.is_nan() || self.playback <= 0.0 {
            return err("playback must be positive");
        }
        let lattice_size = |size: usize| {
            if (4..=256).contains(&size) {
                Ok(size)
            } else {
                err("a lattice is between 4 and 256 spins on a side")
            }
        };
        let state = match self.model {
            Model::Ising {
                size,
                temperature,
                field,
                ordered,
                ..
            } => {
                if temperature.is_nan() || temperature < 0.0 {
                    return err("a temperature cannot be below absolute zero");
                }
                State::Lattice {
                    lattice: Ising::new(
                        lattice_size(size)?,
                        temperature,
                        field,
                        ordered,
                        self.seed,
                    ),
                    averages: Averages::default(),
                    points: Vec::new(),
                    spent: 0,
                }
            }
            Model::Sweep {
                size,
                from,
                to,
                points,
                measure,
                ..
            } => {
                if from.min(to) <= 0.0 || from.is_nan() || to.is_nan() {
                    return err("a sweep runs between two temperatures above zero");
                }
                if points < 2 || measure == 0 {
                    return err(
                        "a sweep needs at least two temperatures and some sweeps to measure in",
                    );
                }
                // Begun in order when it starts cold and in disorder when it starts hot: close to equilibrium
                // either way.
                let ordered = from < to;
                State::Lattice {
                    lattice: Ising::new(lattice_size(size)?, from, 0.0, ordered, self.seed),
                    averages: Averages::default(),
                    points: Vec::new(),
                    spent: 0,
                }
            }
            Model::Darts { batch } => {
                if batch == 0 {
                    return err("darts are thrown at least one at a time");
                }
                State::Darts(Darts::new(DARTS_SHOWN, self.seed))
            }
            Model::Walk { walkers, steps } => {
                if walkers == 0 || walkers > 100_000 || steps == 0 {
                    return err("a walk needs between 1 and 100 000 walkers and at least one step");
                }
                State::Walk(Walkers::new(walkers, self.seed))
            }
        };
        let mut run = Run {
            scene: self.clone(),
            state,
            steps: 0,
            history: Vec::new(),
        };
        run.record();
        Ok(run)
    }
}

impl Run {
    /// True once a sweep has measured its last temperature, or the walkers have taken their last step.
    pub fn finished(&self) -> bool {
        match (&self.scene.model, &self.state) {
            (Model::Sweep { points, .. }, State::Lattice { points: done, .. }) => {
                done.len() >= *points
            }
            (Model::Walk { steps, .. }, State::Walk(walkers)) => walkers.steps >= *steps,
            _ => false,
        }
    }

    pub fn advance(&mut self, steps: usize) {
        for _ in 0..steps {
            if self.finished() {
                return;
            }
            self.step();
            self.steps += 1;
            // Kept sparse enough that a long run still draws quickly.
            let every = 1 + self.steps / 1500;
            if self.steps.is_multiple_of(every) {
                self.record();
            }
        }
    }

    fn step(&mut self) {
        match (&mut self.state, self.scene.model) {
            (
                State::Lattice {
                    lattice, averages, ..
                },
                Model::Ising { settle, .. },
            ) => {
                lattice.sweep();
                if lattice.sweeps > settle {
                    averages.add(lattice);
                }
            }
            (
                State::Lattice {
                    lattice,
                    averages,
                    points,
                    spent,
                },
                Model::Sweep {
                    from,
                    to,
                    points: wanted,
                    settle,
                    measure,
                    ..
                },
            ) => {
                lattice.sweep();
                *spent += 1;
                if *spent > settle {
                    averages.add(lattice);
                }
                if *spent >= settle + measure {
                    let spins = (lattice.size * lattice.size) as f64;
                    let t = lattice.temperature;
                    points.push(Point {
                        temperature: t,
                        magnetisation: averages.magnetisation(),
                        energy: averages.energy(),
                        specific_heat: averages.specific_heat(spins, t),
                        susceptibility: averages.susceptibility(spins, t),
                    });
                    *averages = Averages::default();
                    *spent = 0;
                    if points.len() < wanted {
                        let share = points.len() as f64 / (wanted - 1) as f64;
                        lattice.temperature = from + (to - from) * share;
                    }
                }
            }
            (State::Darts(darts), Model::Darts { batch }) => darts.throw(batch),
            (State::Walk(walkers), _) => walkers.step(),
            _ => {}
        }
    }

    fn record(&mut self) {
        let row = match &self.state {
            State::Lattice { lattice, .. } => [
                lattice.sweeps as f64,
                lattice.magnetisation(),
                lattice.energy(),
            ],
            State::Darts(darts) => [
                darts.thrown as f64,
                darts.estimate(),
                darts.expected_error(),
            ],
            State::Walk(walkers) => [
                walkers.steps as f64,
                walkers.rms_distance(),
                (walkers.steps as f64).sqrt(),
            ],
        };
        self.history.push(row);
    }

    /// Each answer's name, value, what the run gives for it so far, and how close counts as agreeing.
    pub fn answers(&self) -> Vec<(String, f64, Option<f64>, f64)> {
        self.scene
            .answers
            .iter()
            .map(|answer| {
                let measured = answer.measure.and_then(|m| self.measure(m));
                (answer.name.clone(), answer.value, measured, answer.within)
            })
            .collect()
    }

    fn measure(&self, measure: MeasureSpec) -> Option<f64> {
        match (&self.state, measure) {
            (State::Lattice { averages, .. }, MeasureSpec::Magnetisation) => {
                (averages.count >= 20.0).then(|| averages.magnetisation())
            }
            (State::Lattice { averages, .. }, MeasureSpec::Energy) => {
                (averages.count >= 20.0).then(|| averages.energy())
            }
            (State::Lattice { points, .. }, MeasureSpec::HeatPeak) => {
                if !self.finished() {
                    return None;
                }
                points
                    .iter()
                    .max_by(|a, b| a.specific_heat.total_cmp(&b.specific_heat))
                    .map(|p| p.temperature)
            }
            (State::Darts(darts), MeasureSpec::Estimate) => {
                (darts.thrown > 0).then(|| darts.estimate())
            }
            (State::Walk(walkers), MeasureSpec::RmsDistance) => {
                self.finished().then(|| walkers.rms_distance())
            }
            (State::Walk(walkers), MeasureSpec::Drift) => {
                let [x, y] = walkers.mean_position();
                self.finished().then(|| x.hypot(y))
            }
            _ => None,
        }
    }
}
