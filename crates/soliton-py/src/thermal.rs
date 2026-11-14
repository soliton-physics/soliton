use numpy::{PyArray1, PyArray2, PyArrayMethods};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use soliton_thermal::scenario::{catalogue, find, Params};
use soliton_thermal::scene::{Experiment, ThermalScene};
use soliton_thermal::METRES_PER_SECOND;

/// The thermal catalogue as JSON: ids, titles, syllabus and parameter schemas.
#[pyfunction]
pub fn thermal_catalogue_json() -> String {
    let specs: Vec<_> = catalogue().iter().map(|s| s.spec).collect();
    serde_json::to_string(&specs).expect("catalogue serialises")
}

/// A gas of molecules being run through the stages of a scene (design 014).
#[pyclass(name = "Gas")]
pub struct GasExperiment {
    experiment: Experiment,
}

impl GasExperiment {
    fn build(scene: ThermalScene) -> PyResult<Self> {
        let experiment = scene
            .start()
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self { experiment })
    }
}

#[pymethods]
impl GasExperiment {
    /// A catalogue scenario with chosen parameters, e.g. `Gas.scenario("ideal-gas", temperature=500)`.
    #[staticmethod]
    #[pyo3(signature = (id, **params))]
    fn scenario(id: &str, params: Option<&Bound<'_, PyDict>>) -> PyResult<Self> {
        let scenario =
            find(id).ok_or_else(|| PyValueError::new_err(format!("unknown scenario `{id}`")))?;
        let mut chosen = Params::default();
        if let Some(params) = params {
            for (key, value) in params.iter() {
                chosen = chosen.set(&key.extract::<String>()?, value.extract::<f64>()?);
            }
        }
        Self::build(scenario.scene(&chosen))
    }

    #[staticmethod]
    fn from_json(text: &str) -> PyResult<Self> {
        let scene =
            ThermalScene::from_json(text).map_err(|e| PyValueError::new_err(e.to_string()))?;
        Self::build(scene)
    }

    fn to_json(&self) -> String {
        self.experiment.scene.to_json()
    }

    /// Runs the gas on by this many picoseconds, moving through the scene's stages as each ends.
    fn advance(&mut self, picoseconds: f64) {
        self.experiment.advance(picoseconds);
    }

    /// Picoseconds since the start.
    #[getter]
    fn time(&self) -> f64 {
        self.experiment.gas.time
    }

    /// The stage in progress, counted from zero.
    #[getter]
    fn stage(&self) -> usize {
        self.experiment.stage()
    }

    /// True once the last stage has ended, for a scene whose last stage has an end.
    #[getter]
    fn finished(&self) -> bool {
        self.experiment.finished()
    }

    /// Kelvin: two thirds of the mean kinetic energy of a molecule, over Boltzmann's constant.
    #[getter]
    fn temperature(&self) -> f64 {
        self.experiment.gas.temperature()
    }

    /// nm³.
    #[getter]
    fn volume(&self) -> f64 {
        self.experiment.gas.volume()
    }

    /// The textbook's answers: `{name: value}`.
    fn answers<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let out = PyDict::new(py);
        for (name, value, _, _) in self.experiment.answers() {
            out.set_item(name, value)?;
        }
        Ok(out)
    }

    /// What the molecules give so far for each answer they can be asked for: `{name: value}`.
    fn measured<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let out = PyDict::new(py);
        for (name, _, measured, _) in self.experiment.answers() {
            if let Some(value) = measured {
                out.set_item(name, value)?;
            }
        }
        Ok(out)
    }

    /// Positions of the molecules, `(N, 3)`, nm.
    fn positions<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyArray2<f64>>> {
        let molecules = &self.experiment.gas.molecules;
        let flat: Vec<f64> = molecules.iter().flat_map(|m| m.position).collect();
        PyArray1::from_vec(py, flat).reshape([molecules.len(), 3])
    }

    /// Velocities of the molecules, `(N, 3)`, m/s.
    fn velocities<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyArray2<f64>>> {
        let molecules = &self.experiment.gas.molecules;
        let flat: Vec<f64> = molecules
            .iter()
            .flat_map(|m| m.velocity.map(|v| v * METRES_PER_SECOND))
            .collect();
        PyArray1::from_vec(py, flat).reshape([molecules.len(), 3])
    }

    /// Which kind each molecule is, `(N,)`, numbered in the order of the scene's groups.
    fn species<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<i64>> {
        let kinds: Vec<i64> = self
            .experiment
            .gas
            .molecules
            .iter()
            .map(|m| i64::from(m.species))
            .collect();
        PyArray1::from_vec(py, kinds)
    }

    /// The record kept once a picosecond, as arrays: `time` (ps), `volume` (nm³), `pressure` (atm, over the
    /// last forty picoseconds), `temperature` (K), and running totals of `work`, `heat` and `energy`
    /// (10⁻²¹ J).
    fn series<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let samples = &self.experiment.samples;
        let out = PyDict::new(py);
        let column = |pick: fn(&soliton_thermal::scene::Sample) -> f64| {
            PyArray1::from_vec(py, samples.iter().map(pick).collect())
        };
        out.set_item("time", column(|s| s.time))?;
        out.set_item("volume", column(|s| s.volume))?;
        out.set_item("pressure", column(|s| s.pressure))?;
        out.set_item("temperature", column(|s| s.temperature))?;
        out.set_item("work", column(|s| s.work))?;
        out.set_item("heat", column(|s| s.heat))?;
        out.set_item("energy", column(|s| s.energy))?;
        Ok(out)
    }
}
