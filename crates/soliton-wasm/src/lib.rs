use js_sys::{Float64Array, Object, Reflect};
use num_complex::Complex64;
use soliton_fields::exact::{kdv_soliton, nls_bright_soliton};
use soliton_fields::{Kdv, Nls, PeriodicGrid, Simulate, Simulation};
use wasm_bindgen::prelude::*;

enum Engine {
    Kdv(Simulation<Kdv>),
    Nls(Simulation<Nls>),
}

macro_rules! dispatch {
    ($engine:expr, |$sim:ident| $body:expr) => {
        match $engine {
            Engine::Kdv($sim) => $body,
            Engine::Nls($sim) => $body,
        }
    };
}

/// A running simulation of one of the supported wave equations.
#[wasm_bindgen]
pub struct WaveSimulation {
    engine: Engine,
}

#[wasm_bindgen]
impl WaveSimulation {
    /// `model` is `"kdv"` or `"nls"`.
    #[wasm_bindgen(constructor)]
    pub fn new(
        model: &str,
        points: usize,
        length: f64,
        dt: f64,
    ) -> Result<WaveSimulation, JsError> {
        let grid = PeriodicGrid::new(points, length);
        let engine = match model {
            "kdv" => Engine::Kdv(Simulation::new(grid, Kdv, dt)),
            "nls" => Engine::Nls(Simulation::new(grid, Nls, dt)),
            other => return Err(JsError::new(&format!("unknown model `{other}`"))),
        };
        Ok(Self { engine })
    }

    pub fn points(&self) -> usize {
        dispatch!(&self.engine, |s| s.grid().points())
    }

    pub fn length(&self) -> f64 {
        dispatch!(&self.engine, |s| s.grid().length())
    }

    pub fn dt(&self) -> f64 {
        dispatch!(&self.engine, |s| s.dt())
    }

    pub fn time(&self) -> f64 {
        dispatch!(&self.engine, |s| s.time())
    }

    pub fn positions(&self) -> Float64Array {
        Float64Array::from(dispatch!(&self.engine, |s| s.grid().positions()).as_slice())
    }

    pub fn step(&mut self, steps: usize) {
        dispatch!(&mut self.engine, |s| s.advance(steps))
    }

    pub fn clear(&mut self) {
        dispatch!(&mut self.engine, |s| {
            s.set_profile(|_| Complex64::default());
            s.reset_time();
        })
    }

    /// Sets the state from interleaved-free real and imaginary arrays.
    pub fn set_state(&mut self, re: &[f64], im: &[f64]) -> Result<(), JsError> {
        if re.len() != self.points() || im.len() != self.points() {
            return Err(JsError::new("state arrays must match the grid size"));
        }
        let field: Vec<Complex64> = re
            .iter()
            .zip(im)
            .map(|(&r, &i)| Complex64::new(r, i))
            .collect();
        dispatch!(&mut self.engine, |s| s.set_state(&field));
        Ok(())
    }

    pub fn add_kdv_soliton(&mut self, speed: f64, center: f64) {
        dispatch!(&mut self.engine, |s| s.add_profile(|x| Complex64::new(
            kdv_soliton(x, 0.0, speed, center),
            0.0
        )))
    }

    pub fn add_nls_soliton(&mut self, amplitude: f64, velocity: f64, center: f64) {
        dispatch!(&mut self.engine, |s| s.add_profile(|x| nls_bright_soliton(
            x, 0.0, amplitude, velocity, center
        )))
    }

    pub fn real(&mut self) -> Float64Array {
        self.map_state(|z| z.re)
    }

    pub fn imag(&mut self) -> Float64Array {
        self.map_state(|z| z.im)
    }

    pub fn magnitude(&mut self) -> Float64Array {
        self.map_state(|z| z.norm())
    }

    /// Conserved quantities as a plain `{ name: value }` object.
    pub fn invariants(&mut self) -> Object {
        let object = Object::new();
        for invariant in dispatch!(&mut self.engine, |s| s.invariants()) {
            Reflect::set(&object, &invariant.name.into(), &invariant.value.into()).unwrap();
        }
        object
    }

    fn map_state(&mut self, f: impl Fn(&Complex64) -> f64) -> Float64Array {
        let values: Vec<f64> = dispatch!(&mut self.engine, |s| s.state())
            .iter()
            .map(f)
            .collect();
        Float64Array::from(values.as_slice())
    }
}
