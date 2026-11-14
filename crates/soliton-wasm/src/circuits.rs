//! A circuit as the playground sees it (design 015): solved, stepped and read.

use js_sys::Float64Array;
use serde::Serialize;
use soliton_circuits::scenario::{catalogue, find, Params};
use soliton_circuits::scene::{Analysis, Bench, CircuitScene, Quantity};
use wasm_bindgen::prelude::*;

fn to_js<T: Serialize>(value: &T) -> Result<JsValue, JsError> {
    value
        .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .map_err(|e| JsError::new(&e.to_string()))
}

fn failed(error: impl std::fmt::Display) -> JsError {
    JsError::new(&error.to_string())
}

/// The circuit scenarios with their parameter schemas.
#[wasm_bindgen]
pub fn circuits_catalogue() -> Result<JsValue, JsError> {
    let specs: Vec<soliton_circuits::scenario::ScenarioSpec> =
        catalogue().iter().map(|s| s.spec).collect();
    to_js(&specs)
}

#[derive(Serialize)]
struct AnswerInfo {
    name: String,
    value: f64,
    /// What the solved circuit gives, where it can give it.
    measured: Option<f64>,
}

#[wasm_bindgen]
pub struct CircuitBench {
    bench: Bench,
}

#[wasm_bindgen]
impl CircuitBench {
    /// `params` is a plain object `{ name: value }`; missing entries take the scenario defaults.
    #[wasm_bindgen(constructor)]
    pub fn new(id: &str, params: JsValue) -> Result<CircuitBench, JsError> {
        let scenario = find(id).ok_or_else(|| JsError::new(&format!("unknown scenario `{id}`")))?;
        let chosen: std::collections::HashMap<String, f64> =
            if params.is_undefined() || params.is_null() {
                Default::default()
            } else {
                serde_wasm_bindgen::from_value(params).map_err(failed)?
            };
        let mut p = Params::default();
        for (name, value) in chosen {
            p = p.set(&name, value);
        }
        Self::from_file(scenario.scene(&p))
    }

    pub fn from_scene(json: &str) -> Result<CircuitBench, JsError> {
        Self::from_file(CircuitScene::from_json(json).map_err(failed)?)
    }

    fn from_file(scene: CircuitScene) -> Result<CircuitBench, JsError> {
        Ok(Self {
            bench: scene.start().map_err(failed)?,
        })
    }

    pub fn title(&self) -> String {
        self.bench.scene.title.clone()
    }

    /// The circuit as data: junctions and where they are drawn, components, how it is solved.
    pub fn scene_json(&self) -> String {
        self.bench.scene.to_json()
    }

    /// `steady`, `transient` or `alternating`.
    pub fn analysis(&self) -> String {
        match self.bench.scene.analysis {
            Analysis::Steady => "steady",
            Analysis::Transient { .. } => "transient",
            Analysis::Alternating { .. } => "alternating",
        }
        .to_string()
    }

    /// Seconds of the circuit to show in one second of watching.
    pub fn playback(&self) -> f64 {
        self.bench.playback()
    }

    /// How long a circuit followed in time runs for, s; zero for any other.
    pub fn duration(&self) -> f64 {
        match self.bench.scene.analysis {
            Analysis::Transient { duration, .. } => duration,
            _ => 0.0,
        }
    }

    /// The frequency of the alternating sources, Hz; zero when there are none.
    pub fn frequency(&self) -> f64 {
        self.bench.circuit.frequency().unwrap_or(0.0)
    }

    pub fn time(&self) -> f64 {
        self.bench.state.time
    }

    /// Moves time on by this many seconds of the circuit.
    pub fn advance(&mut self, seconds: f64) -> Result<(), JsError> {
        self.bench.advance(seconds).map_err(failed)
    }

    /// True once a circuit followed in time has run its course.
    pub fn finished(&self) -> bool {
        self.bench.finished()
    }

    /// `voltage, current` for each component now: volts with its `a` above its `b`, amperes from `a` to `b`.
    pub fn readings(&self) -> Float64Array {
        let flat: Vec<f64> = self
            .bench
            .state
            .readings
            .iter()
            .flat_map(|r| [r.voltage, r.current])
            .collect();
        Float64Array::from(flat.as_slice())
    }

    /// The complex amplitudes of each component's alternating voltage and current, as
    /// `Re V, Im V, Re I, Im I`; empty for a circuit not solved that way. Each quantity is the imaginary part
    /// of its amplitude times `e^{iωt}`.
    pub fn phasors(&self) -> Float64Array {
        let flat: Vec<f64> = self
            .bench
            .phasors
            .iter()
            .flatten()
            .flat_map(|p| [p.voltage.re, p.voltage.im, p.current.re, p.current.im])
            .collect();
        Float64Array::from(flat.as_slice())
    }

    /// The times of every step so far, for a circuit followed in time.
    pub fn series_time(&self) -> Float64Array {
        let times: Vec<f64> = self.bench.history.iter().map(|s| s.time).collect();
        Float64Array::from(times.as_slice())
    }

    /// One component's `voltage` or `current` at every step so far.
    pub fn series(&self, element: usize, quantity: &str) -> Result<Float64Array, JsError> {
        let quantity = match quantity {
            "voltage" => Quantity::Voltage,
            "current" => Quantity::Current,
            other => return Err(JsError::new(&format!("unknown quantity `{other}`"))),
        };
        if element >= self.bench.circuit.elements.len() {
            return Err(JsError::new("no such component"));
        }
        Ok(Float64Array::from(
            self.bench.series(element, quantity).as_slice(),
        ))
    }

    /// `[{ name, value, measured }]`: the textbook's answers and what the solved circuit gives.
    pub fn answers(&self) -> Result<JsValue, JsError> {
        let infos: Vec<AnswerInfo> = self
            .bench
            .answers()
            .map_err(failed)?
            .into_iter()
            .map(|(name, value, measured)| AnswerInfo {
                name,
                value,
                measured,
            })
            .collect();
        to_js(&infos)
    }
}
