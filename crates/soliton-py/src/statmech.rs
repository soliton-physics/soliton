use numpy::{PyArray1, PyArray2, PyArrayMethods};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use soliton_statmech::scenario::{catalogue, find, Params};
use soliton_statmech::scene::{Run, StatScene, State};

/// The statistical catalogue as JSON: ids, titles and parameter schemas.
#[pyfunction]
pub fn statmech_catalogue_json() -> String {
    let specs: Vec<_> = catalogue().iter().map(|s| s.spec).collect();
    serde_json::to_string(&specs).expect("catalogue serialises")
}

/// Onsager's exact magnetisation per spin of the square-lattice Ising model at a temperature in `J/k`.
#[pyfunction]
pub fn onsager_magnetisation(temperature: f64) -> f64 {
    soliton_statmech::onsager_magnetisation(temperature)
}

/// Onsager's exact energy per spin of the square-lattice Ising model, in units of `J`.
#[pyfunction]
pub fn onsager_energy(temperature: f64) -> f64 {
    soliton_statmech::onsager_energy(temperature)
}

fn failed(error: impl std::fmt::Display) -> PyErr {
    PyValueError::new_err(error.to_string())
}

/// A model run by chance (design 016): an Ising lattice, darts at a square, or random walkers.
#[pyclass(name = "StatRun")]
pub struct StatRun {
    run: Run,
}

#[pymethods]
impl StatRun {
    /// A catalogue scenario with chosen parameters, e.g. `Run.scenario("ising-model", temperature=2.0)`.
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
        Ok(Self {
            run: scenario.scene(&chosen).start().map_err(failed)?,
        })
    }

    #[staticmethod]
    fn from_json(text: &str) -> PyResult<Self> {
        let scene = StatScene::from_json(text).map_err(failed)?;
        Ok(Self {
            run: scene.start().map_err(failed)?,
        })
    }

    fn to_json(&self) -> String {
        self.run.scene.to_json()
    }

    /// Makes this many steps: sweeps of a lattice, batches of darts, or steps of every walker.
    fn advance(&mut self, steps: usize) {
        self.run.advance(steps);
    }

    #[getter]
    fn steps(&self) -> u64 {
        self.run.steps
    }

    /// True once a temperature sweep or a walk has run its course.
    #[getter]
    fn finished(&self) -> bool {
        self.run.finished()
    }

    /// The spins of a lattice as an `(L, L)` array of `+1` and `−1`.
    fn spins<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyArray2<i8>>> {
        match &self.run.state {
            State::Lattice { lattice, .. } => {
                PyArray1::from_vec(py, lattice.spins.clone()).reshape([lattice.size, lattice.size])
            }
            _ => Err(PyValueError::new_err("this run has no lattice")),
        }
    }

    /// The temperatures a sweep has measured so far, `(n, 5)`: temperature, magnetisation, energy, specific
    /// heat and susceptibility per spin.
    fn points<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyArray2<f64>>> {
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
                PyArray1::from_vec(py, flat).reshape([points.len(), 5])
            }
            _ => Err(PyValueError::new_err("this run has no lattice")),
        }
    }

    /// Where each walker is, `(n, 2)`, in steps from the start.
    fn walkers<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyArray2<i32>>> {
        match &self.run.state {
            State::Walk(walkers) => {
                let flat: Vec<i32> = walkers.positions.iter().flatten().copied().collect();
                PyArray1::from_vec(py, flat).reshape([walkers.positions.len(), 2])
            }
            _ => Err(PyValueError::new_err("this run has no walkers")),
        }
    }

    /// The thinned record, `(n, 3)`: the step, then magnetisation and energy for a lattice, the estimate of π
    /// and its expected error for darts, or the walkers' rms distance and `√steps`.
    fn history<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyArray2<f64>>> {
        let flat: Vec<f64> = self.run.history.iter().flatten().copied().collect();
        PyArray1::from_vec(py, flat).reshape([self.run.history.len(), 3])
    }

    /// The exact answers: `{name: value}`.
    fn answers<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let out = PyDict::new(py);
        for (name, value, _, _) in self.run.answers() {
            out.set_item(name, value)?;
        }
        Ok(out)
    }

    /// What the random trials have given so far: `{name: value}`.
    fn measured<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let out = PyDict::new(py);
        for (name, _, measured, _) in self.run.answers() {
            if let Some(value) = measured {
                out.set_item(name, value)?;
            }
        }
        Ok(out)
    }
}
