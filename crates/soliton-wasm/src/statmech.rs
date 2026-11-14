//! A statistical experiment as the playground sees it (design 016).

use js_sys::{Float64Array, Int32Array, Int8Array};
use serde::Serialize;
use soliton_statmech::scenario::{catalogue, find, Params};
use soliton_statmech::scene::{Model, Run, StatScene, State};
use soliton_statmech::{onsager_energy, onsager_magnetisation, CRITICAL_TEMPERATURE};
use wasm_bindgen::prelude::*;

fn to_js<T: Serialize>(value: &T) -> Result<JsValue, JsError> {
    value
        .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .map_err(|e| JsError::new(&e.to_string()))
}

/// The statistical scenarios with their parameter schemas.
#[wasm_bindgen]
pub fn statmech_catalogue() -> Result<JsValue, JsError> {
    let specs: Vec<soliton_statmech::scenario::ScenarioSpec> =
        catalogue().iter().map(|s| s.spec).collect();
    to_js(&specs)
}

#[derive(Serialize)]
struct AnswerInfo {
    name: String,
    value: f64,
    measured: Option<f64>,
    /// How close, in the answer's own units, counts as agreeing.
    within: f64,
}

/// What the run reads at this moment. Fields that do not apply to the model in hand are zero.
#[derive(Serialize, Default)]
struct NowInfo {
    steps: f64,
    /// A lattice: its temperature and field, its magnetisation and energy per spin now, and their averages
    /// since it settled.
    temperature: f64,
    field: f64,
    magnetisation: f64,
    energy: f64,
    mean_magnetisation: f64,
    mean_energy: f64,
    /// Darts: how many have been thrown, the estimate of π and the error to expect in it.
    thrown: f64,
    estimate: f64,
    expected_error: f64,
    /// Walkers: their root mean square distance from the start, and `√steps`.
    rms_distance: f64,
    root_steps: f64,
}

#[wasm_bindgen]
pub struct StatRun {
    run: Run,
}

#[wasm_bindgen]
impl StatRun {
    /// `params` is a plain object `{ name: value }`; missing entries take the scenario defaults.
    #[wasm_bindgen(constructor)]
    pub fn new(id: &str, params: JsValue) -> Result<StatRun, JsError> {
        let scenario = find(id).ok_or_else(|| JsError::new(&format!("unknown scenario `{id}`")))?;
        let chosen: std::collections::HashMap<String, f64> =
            if params.is_undefined() || params.is_null() {
                Default::default()
            } else {
                serde_wasm_bindgen::from_value(params).map_err(|e| JsError::new(&e.to_string()))?
            };
        let mut p = Params::default();
        for (name, value) in chosen {
            p = p.set(&name, value);
        }
        Self::from_file(scenario.scene(&p))
    }

    pub fn from_scene(json: &str) -> Result<StatRun, JsError> {
        Self::from_file(StatScene::from_json(json).map_err(|e| JsError::new(&e.to_string()))?)
    }

    fn from_file(scene: StatScene) -> Result<StatRun, JsError> {
        Ok(Self {
            run: scene.start().map_err(|e| JsError::new(&e.to_string()))?,
        })
    }

    pub fn title(&self) -> String {
        self.run.scene.title.clone()
    }

    pub fn scene_json(&self) -> String {
        self.run.scene.to_json()
    }

    /// `ising`, `sweep`, `darts` or `walk`.
    pub fn kind(&self) -> String {
        match self.run.scene.model {
            Model::Ising { .. } => "ising",
            Model::Sweep { .. } => "sweep",
            Model::Darts { .. } => "darts",
            Model::Walk { .. } => "walk",
        }
        .to_string()
    }

    /// Steps to make in one second of watching.
    pub fn playback(&self) -> f64 {
        self.run.scene.playback
    }

    /// One step is a sweep of the lattice, a batch of darts, or a step of every walker.
    pub fn advance(&mut self, steps: usize) {
        self.run.advance(steps);
    }

    pub fn finished(&self) -> bool {
        self.run.finished()
    }

    /// Spins along a side of the lattice; zero when there is none.
    pub fn size(&self) -> usize {
        match &self.run.state {
            State::Lattice { lattice, .. } => lattice.size,
            _ => 0,
        }
    }

    /// The spins row by row, each `+1` or `−1`.
    pub fn spins(&self) -> Int8Array {
        match &self.run.state {
            State::Lattice { lattice, .. } => Int8Array::from(lattice.spins.as_slice()),
            _ => Int8Array::new_with_length(0),
        }
    }

    /// The most recent darts as `x, y, inside` triples.
    pub fn darts(&self) -> Float64Array {
        match &self.run.state {
            State::Darts(darts) => {
                let flat: Vec<f64> = darts.recent.iter().flatten().copied().collect();
                Float64Array::from(flat.as_slice())
            }
            _ => Float64Array::new_with_length(0),
        }
    }

    /// Where each walker is, as `x, y` pairs in steps from the start.
    pub fn walkers(&self) -> Int32Array {
        match &self.run.state {
            State::Walk(walkers) => {
                let flat: Vec<i32> = walkers.positions.iter().flatten().copied().collect();
                Int32Array::from(flat.as_slice())
            }
            _ => Int32Array::new_with_length(0),
        }
    }

    pub fn now(&self) -> Result<JsValue, JsError> {
        let mut info = NowInfo {
            steps: self.run.steps as f64,
            ..NowInfo::default()
        };
        match &self.run.state {
            State::Lattice {
                lattice, averages, ..
            } => {
                info.temperature = lattice.temperature;
                info.field = lattice.field;
                info.magnetisation = lattice.magnetisation();
                info.energy = lattice.energy();
                info.mean_magnetisation = averages.magnetisation();
                info.mean_energy = averages.energy();
            }
            State::Darts(darts) => {
                info.thrown = darts.thrown as f64;
                info.estimate = darts.estimate();
                info.expected_error = darts.expected_error();
            }
            State::Walk(walkers) => {
                info.rms_distance = walkers.rms_distance();
                info.root_steps = (walkers.steps as f64).sqrt();
            }
        }
        to_js(&info)
    }

    /// The thinned record for graphs, as `step, first, second` triples: magnetisation and energy for a
    /// lattice, the estimate and its expected error for darts, the walkers' distance and `√steps` for a walk.
    pub fn history(&self) -> Float64Array {
        let flat: Vec<f64> = self.run.history.iter().flatten().copied().collect();
        Float64Array::from(flat.as_slice())
    }

    /// The temperatures a sweep has measured so far, as
    /// `temperature, magnetisation, energy, specific heat, susceptibility` for each.
    pub fn points(&self) -> Float64Array {
        match &self.run.state {
            State::Lattice { points, .. } => {
                let flat: Vec<f64> = points
                    .iter()
                    .flat_map(|p| {
                        [
                            p.temperature,
                            p.magnetisation,
                            p.energy,
                            p.specific_heat,
                            p.susceptibility,
                        ]
                    })
                    .collect();
                Float64Array::from(flat.as_slice())
            }
            _ => Float64Array::new_with_length(0),
        }
    }

    /// Onsager's exact results for an infinite lattice, at `count` temperatures from `from` to `to`, as
    /// `temperature, magnetisation, energy` triples.
    pub fn onsager(from: f64, to: f64, count: usize) -> Float64Array {
        let flat: Vec<f64> = (0..count)
            .flat_map(|i| {
                let t = from + (to - from) * i as f64 / (count.max(2) - 1) as f64;
                [t, onsager_magnetisation(t), onsager_energy(t)]
            })
            .collect();
        Float64Array::from(flat.as_slice())
    }

    /// The critical temperature of the square lattice, `2/ln(1 + √2)`.
    pub fn critical_temperature() -> f64 {
        CRITICAL_TEMPERATURE
    }

    /// `[{ name, value, measured, within }]`.
    pub fn answers(&self) -> Result<JsValue, JsError> {
        let infos: Vec<AnswerInfo> = self
            .run
            .answers()
            .into_iter()
            .map(|(name, value, measured, within)| AnswerInfo {
                name,
                value,
                measured,
                within,
            })
            .collect();
        to_js(&infos)
    }
}
