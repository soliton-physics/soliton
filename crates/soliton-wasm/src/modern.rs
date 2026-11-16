//! A modern physics lesson as the playground sees it (design 017).

use serde::Serialize;
use soliton_modern::scenario::{catalogue, find, Params, Run, ScenarioSpec};
use wasm_bindgen::prelude::*;

fn to_js<T: Serialize>(value: &T) -> Result<JsValue, JsError> {
    value
        .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .map_err(|e| JsError::new(&e.to_string()))
}

/// The modern physics scenarios with their parameter schemas.
#[wasm_bindgen]
pub fn modern_catalogue() -> Result<JsValue, JsError> {
    let specs: Vec<ScenarioSpec> = catalogue().iter().map(|s| s.spec).collect();
    to_js(&specs)
}

#[wasm_bindgen]
pub struct ModernRun {
    run: Run,
}

#[wasm_bindgen]
impl ModernRun {
    /// `params` is a plain object `{ name: value }`; missing entries take the scenario defaults.
    #[wasm_bindgen(constructor)]
    pub fn new(id: &str, params: JsValue) -> Result<ModernRun, JsError> {
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
        Ok(Self {
            run: scenario.start(&p),
        })
    }

    /// Moves on by this many seconds of watching.
    pub fn advance(&mut self, seconds: f64) {
        self.run.advance(seconds);
    }

    pub fn finished(&self) -> bool {
        self.run.finished()
    }

    /// True for a lesson with nothing to play.
    pub fn still(&self) -> bool {
        self.run.still()
    }

    pub fn clock(&self) -> String {
        self.run.clock()
    }

    /// What stands on the stage: `{ kind, ... }`.
    pub fn picture(&self) -> Result<JsValue, JsError> {
        to_js(&self.run.picture())
    }

    /// `[{ label, value, unit }]`.
    pub fn readings(&self) -> Result<JsValue, JsError> {
        to_js(&self.run.readings())
    }

    /// `[{ title, x_label, y_label, curves, marker, from_zero, ladder }]`.
    pub fn plots(&self) -> Result<JsValue, JsError> {
        to_js(&self.run.plots())
    }

    /// `[{ name, value, measured, within }]`.
    pub fn answers(&self) -> Result<JsValue, JsError> {
        to_js(&self.run.answers())
    }
}
