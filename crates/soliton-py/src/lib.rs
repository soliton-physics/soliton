use num_complex::Complex64;
use numpy::{IntoPyArray, PyArray1, PyReadonlyArray1};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;
mod circuits;
mod mechanics;
mod optics;
mod statmech;
mod thermal;

use soliton_fields::{exact, Kdv, Nls, PeriodicGrid, Simulate};

enum Engine {
    Kdv(soliton_fields::Simulation<Kdv>),
    Nls(soliton_fields::Simulation<Nls>),
}

macro_rules! dispatch {
    ($engine:expr, |$sim:ident| $body:expr) => {
        match $engine {
            Engine::Kdv($sim) => $body,
            Engine::Nls($sim) => $body,
        }
    };
}

/// Simulation(model, points, length, dt)
///
/// `model` is "kdv" or "nls". The grid is periodic on [-length/2, length/2).
#[pyclass]
struct Simulation {
    engine: Engine,
}

#[pymethods]
impl Simulation {
    #[new]
    #[pyo3(signature = (model, points, length, dt))]
    fn new(model: &str, points: usize, length: f64, dt: f64) -> PyResult<Self> {
        let grid = PeriodicGrid::new(points, length);
        let engine = match model {
            "kdv" => Engine::Kdv(soliton_fields::Simulation::new(grid, Kdv, dt)),
            "nls" => Engine::Nls(soliton_fields::Simulation::new(grid, Nls, dt)),
            other => return Err(PyValueError::new_err(format!("unknown model `{other}`"))),
        };
        Ok(Self { engine })
    }

    #[getter]
    fn time(&self) -> f64 {
        dispatch!(&self.engine, |s| s.time())
    }

    #[getter]
    fn dt(&self) -> f64 {
        dispatch!(&self.engine, |s| s.dt())
    }

    #[getter]
    fn points(&self) -> usize {
        dispatch!(&self.engine, |s| s.grid().points())
    }

    #[getter]
    fn length(&self) -> f64 {
        dispatch!(&self.engine, |s| s.grid().length())
    }

    /// Grid positions as a float64 array.
    #[getter]
    fn x<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        dispatch!(&self.engine, |s| s.grid().positions()).into_pyarray(py)
    }

    /// Current field in physical space as a complex128 array.
    #[getter]
    fn state<'py>(&mut self, py: Python<'py>) -> Bound<'py, PyArray1<Complex64>> {
        dispatch!(&mut self.engine, |s| s.state()).into_pyarray(py)
    }

    #[setter]
    fn set_state(&mut self, field: PyReadonlyArray1<Complex64>) -> PyResult<()> {
        let field = field.as_slice()?;
        if field.len() != self.points() {
            return Err(PyValueError::new_err("state length must match the grid"));
        }
        dispatch!(&mut self.engine, |s| s.set_state(field));
        Ok(())
    }

    /// Spectrum in unnormalised forward-FFT order.
    #[getter]
    fn spectrum<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<Complex64>> {
        dispatch!(&self.engine, |s| s.spectrum().to_vec()).into_pyarray(py)
    }

    #[pyo3(signature = (steps = 1))]
    fn step(&mut self, steps: usize) {
        dispatch!(&mut self.engine, |s| s.advance(steps))
    }

    fn reset_time(&mut self) {
        dispatch!(&mut self.engine, |s| s.reset_time())
    }

    /// Conserved quantities as a dict.
    fn invariants<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        for invariant in dispatch!(&mut self.engine, |s| s.invariants()) {
            dict.set_item(invariant.name, invariant.value)?;
        }
        Ok(dict)
    }
}

/// kdv_soliton(x, t, speed, center) -> float64 array
#[pyfunction]
fn kdv_soliton<'py>(
    py: Python<'py>,
    x: PyReadonlyArray1<f64>,
    t: f64,
    speed: f64,
    center: f64,
) -> PyResult<Bound<'py, PyArray1<Complex64>>> {
    let values: Vec<Complex64> = x
        .as_slice()?
        .iter()
        .map(|&x| Complex64::new(exact::kdv_soliton(x, t, speed, center), 0.0))
        .collect();
    Ok(values.into_pyarray(py))
}

/// nls_bright_soliton(x, t, amplitude, velocity, center) -> complex128 array
#[pyfunction]
fn nls_bright_soliton<'py>(
    py: Python<'py>,
    x: PyReadonlyArray1<f64>,
    t: f64,
    amplitude: f64,
    velocity: f64,
    center: f64,
) -> PyResult<Bound<'py, PyArray1<Complex64>>> {
    let values: Vec<Complex64> = x
        .as_slice()?
        .iter()
        .map(|&x| exact::nls_bright_soliton(x, t, amplitude, velocity, center))
        .collect();
    Ok(values.into_pyarray(py))
}

#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Simulation>()?;
    m.add_function(wrap_pyfunction!(kdv_soliton, m)?)?;
    m.add_function(wrap_pyfunction!(nls_bright_soliton, m)?)?;
    m.add_class::<mechanics::Scene>()?;
    m.add_class::<mechanics::Run>()?;
    m.add_function(wrap_pyfunction!(mechanics::catalogue_json, m)?)?;
    m.add_class::<optics::Bench>()?;
    m.add_function(wrap_pyfunction!(optics::optics_catalogue_json, m)?)?;
    m.add_class::<circuits::CircuitBench>()?;
    m.add_function(wrap_pyfunction!(circuits::circuits_catalogue_json, m)?)?;
    m.add_class::<statmech::StatRun>()?;
    m.add_function(wrap_pyfunction!(statmech::statmech_catalogue_json, m)?)?;
    m.add_function(wrap_pyfunction!(statmech::onsager_magnetisation, m)?)?;
    m.add_function(wrap_pyfunction!(statmech::onsager_energy, m)?)?;
    m.add_class::<thermal::GasExperiment>()?;
    m.add_function(wrap_pyfunction!(thermal::thermal_catalogue_json, m)?)?;
    Ok(())
}
