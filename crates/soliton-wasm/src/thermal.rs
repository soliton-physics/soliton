//! The thermal experiment as the playground sees it (design 014): a gas to step, draw and question.

use js_sys::Float64Array;
use serde::Serialize;
use soliton_thermal::scenario::{catalogue, find, Params};
use soliton_thermal::scene::{maxwell, Experiment, ThermalScene};
use soliton_thermal::{Wall, ATMOSPHERE, JOULES, PASCALS};
use wasm_bindgen::prelude::*;

fn to_js<T: Serialize>(value: &T) -> Result<JsValue, JsError> {
    value
        .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .map_err(|e| JsError::new(&e.to_string()))
}

/// The thermal scenarios with their parameter schemas.
#[wasm_bindgen]
pub fn thermal_catalogue() -> Result<JsValue, JsError> {
    let specs: Vec<soliton_thermal::scenario::ScenarioSpec> =
        catalogue().iter().map(|s| s.spec).collect();
    to_js(&specs)
}

#[derive(Serialize)]
struct GroupInfo {
    name: String,
    count: usize,
    /// amu.
    mass: f64,
    /// nm.
    radius: f64,
}

#[derive(Serialize)]
struct AnswerInfo {
    name: String,
    value: f64,
    /// What the gas gives so far; `null` until its stage has begun.
    measured: Option<f64>,
    /// The fraction within which the two count as agreeing.
    tolerance: f64,
}

#[derive(Serialize)]
struct NowInfo {
    /// ps.
    time: f64,
    /// K.
    temperature: f64,
    /// atm, averaged over the last few picoseconds.
    pressure: f64,
    /// nm³.
    volume: f64,
    /// Totals since the start, 10⁻²¹ J.
    energy: f64,
    work: f64,
    heat: f64,
    /// Collisions between molecules so far.
    encounters: f64,
}

#[wasm_bindgen]
pub struct ThermalGas {
    experiment: Experiment,
}

#[wasm_bindgen]
impl ThermalGas {
    /// `params` is a plain object `{ name: value }`; missing entries take the scenario defaults.
    #[wasm_bindgen(constructor)]
    pub fn new(id: &str, params: JsValue) -> Result<ThermalGas, JsError> {
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

    pub fn from_scene(json: &str) -> Result<ThermalGas, JsError> {
        let scene = ThermalScene::from_json(json).map_err(|e| JsError::new(&e.to_string()))?;
        Self::from_file(scene)
    }

    fn from_file(scene: ThermalScene) -> Result<ThermalGas, JsError> {
        let experiment = scene.start().map_err(|e| JsError::new(&e.to_string()))?;
        Ok(Self { experiment })
    }

    pub fn title(&self) -> String {
        self.experiment.scene.title.clone()
    }

    pub fn scene_json(&self) -> String {
        self.experiment.scene.to_json()
    }

    /// Picoseconds of the gas to show in one second of watching.
    pub fn playback(&self) -> f64 {
        self.experiment.scene.playback
    }

    /// Runs the gas on by this many picoseconds.
    pub fn advance(&mut self, picoseconds: f64) {
        self.experiment.advance(picoseconds);
    }

    /// True once the last stage has ended, for a scene whose last stage has an end.
    pub fn finished(&self) -> bool {
        self.experiment.finished()
    }

    /// The kinds of molecule in the box, in the order of their species numbers.
    pub fn groups(&self) -> Result<JsValue, JsError> {
        let infos: Vec<GroupInfo> = self
            .experiment
            .scene
            .groups
            .iter()
            .map(|g| GroupInfo {
                name: g.name.clone(),
                count: g.count,
                mass: g.mass,
                radius: g.radius,
            })
            .collect();
        to_js(&infos)
    }

    /// Every molecule as `x, y, z, radius, species`, in nm.
    pub fn molecules(&self) -> Float64Array {
        let flat: Vec<f64> = self
            .experiment
            .gas
            .molecules
            .iter()
            .flat_map(|m| {
                [
                    m.position[0],
                    m.position[1],
                    m.position[2],
                    m.radius,
                    f64::from(m.species),
                ]
            })
            .collect();
        Float64Array::from(flat.as_slice())
    }

    /// The box as it is now, `[x, y, z]` in nm; x is where the piston stands.
    pub fn size(&self) -> Float64Array {
        Float64Array::from(self.experiment.gas.size.as_slice())
    }

    /// The temperature each wall is held at, in K, in the order low x, high x, low y, high y, low z, high z;
    /// `NaN` for a wall that is insulated.
    pub fn walls(&self) -> Float64Array {
        let temperatures: Vec<f64> = self
            .experiment
            .gas
            .walls
            .iter()
            .map(|wall| match wall {
                Wall::Bath(t) => *t,
                Wall::Insulating => f64::NAN,
            })
            .collect();
        Float64Array::from(temperatures.as_slice())
    }

    /// Where the partition stands along x, while there is one.
    pub fn partition(&self) -> Option<f64> {
        self.experiment.gas.partition
    }

    /// Whether the piston is moving, and which way: +1 out, −1 in, 0 at rest.
    pub fn piston(&self) -> f64 {
        let speed = self.experiment.gas.piston_speed;
        if speed == 0.0 {
            0.0
        } else {
            speed.signum()
        }
    }

    /// The stage in progress and what it is called.
    pub fn stage(&self) -> usize {
        self.experiment.stage()
    }

    pub fn stage_label(&self) -> String {
        self.experiment.scene.stages[self.experiment.stage()]
            .label
            .clone()
    }

    pub fn stage_count(&self) -> usize {
        self.experiment.scene.stages.len()
    }

    /// The state of the gas at this moment.
    pub fn now(&self) -> Result<JsValue, JsError> {
        let gas = &self.experiment.gas;
        let energy = JOULES * 1e21;
        to_js(&NowInfo {
            time: gas.time,
            temperature: gas.temperature(),
            pressure: self.experiment.samples.last().map_or(0.0, |s| s.pressure),
            volume: gas.volume(),
            energy: gas.kinetic_energy() * energy,
            work: gas.work * energy,
            heat: gas.heat.iter().sum::<f64>() * energy,
            encounters: gas.encounters as f64,
        })
    }

    /// `[{ name, value, measured, tolerance }]`: the textbook's answers and what the gas gives so far.
    pub fn answers(&self) -> Result<JsValue, JsError> {
        let infos: Vec<AnswerInfo> = self
            .experiment
            .answers()
            .into_iter()
            .map(|(name, value, measured, tolerance)| AnswerInfo {
                name,
                value,
                measured,
                tolerance,
            })
            .collect();
        to_js(&infos)
    }

    /// The record kept once a picosecond, one quantity at a time: `time` (ps), `volume` (nm³), `pressure`
    /// (atm), `temperature` (K), `work`, `heat`, `energy` (10⁻²¹ J).
    pub fn series(&self, quantity: &str) -> Result<Float64Array, JsError> {
        let samples = &self.experiment.samples;
        let values: Vec<f64> = match quantity {
            "time" => samples.iter().map(|s| s.time).collect(),
            "volume" => samples.iter().map(|s| s.volume).collect(),
            "pressure" => samples.iter().map(|s| s.pressure).collect(),
            "temperature" => samples.iter().map(|s| s.temperature).collect(),
            "work" => samples.iter().map(|s| s.work).collect(),
            "heat" => samples.iter().map(|s| s.heat).collect(),
            "energy" => samples.iter().map(|s| s.energy).collect(),
            other => return Err(JsError::new(&format!("unknown quantity `{other}`"))),
        };
        Ok(Float64Array::from(values.as_slice()))
    }

    /// The share of one kind of molecule found in the left half of the box, at each sample of the record.
    pub fn series_share(&self, species: usize) -> Float64Array {
        let shares = self
            .experiment
            .shares
            .get(species)
            .map_or(&[][..], Vec::as_slice);
        Float64Array::from(shares)
    }

    /// How one kind of molecule is spread over speed from zero to `fastest` m/s, as a probability density per
    /// m/s in `bins` equal steps. A negative `species` takes every kind together.
    pub fn speed_histogram(&self, species: i32, bins: usize, fastest: f64) -> Float64Array {
        let which = u8::try_from(species).ok();
        let density = self.experiment.speed_histogram(which, bins, fastest);
        Float64Array::from(density.as_slice())
    }

    /// Maxwell's distribution for a molecule of `mass` amu at `temperature` K: the probability density per
    /// m/s at each of `points` speeds evenly spaced from zero to `fastest` m/s.
    pub fn maxwell(mass: f64, temperature: f64, points: usize, fastest: f64) -> Float64Array {
        let curve: Vec<f64> = (0..points)
            .map(|i| {
                let speed = fastest * i as f64 / (points.max(2) - 1) as f64;
                maxwell(mass, temperature, speed)
            })
            .collect();
        Float64Array::from(curve.as_slice())
    }

    /// The pressure an ideal gas of this many molecules would have at this volume and temperature, atm.
    pub fn ideal_pressure(&self) -> f64 {
        let gas = &self.experiment.gas;
        let count = gas.molecules.len() as f64;
        count * soliton_thermal::BOLTZMANN * gas.temperature() / gas.volume() * PASCALS / ATMOSPHERE
    }
}
