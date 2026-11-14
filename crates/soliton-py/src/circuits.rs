use numpy::PyArray1;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyComplex, PyDict};
use soliton_circuits::scenario::{catalogue, find, Params};
use soliton_circuits::scene::{Bench, CircuitScene, Quantity};

/// The circuits catalogue as JSON: ids, titles, syllabus and parameter schemas.
#[pyfunction]
pub fn circuits_catalogue_json() -> String {
    let specs: Vec<_> = catalogue().iter().map(|s| s.spec).collect();
    serde_json::to_string(&specs).expect("catalogue serialises")
}

fn failed(error: impl std::fmt::Display) -> PyErr {
    PyValueError::new_err(error.to_string())
}

/// A circuit solved from Kirchhoff's laws (design 015): steady, followed in time, or alternating.
#[pyclass(name = "Circuit")]
pub struct CircuitBench {
    bench: Bench,
}

impl CircuitBench {
    fn index(&self, name: &str) -> PyResult<usize> {
        self.bench
            .circuit
            .find(name)
            .ok_or_else(|| PyValueError::new_err(format!("no component called `{name}`")))
    }
}

#[pymethods]
impl CircuitBench {
    /// A catalogue scenario with chosen parameters, e.g. `Circuit.scenario("series-lcr", frequency=35.6)`.
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
            bench: scenario.scene(&chosen).start().map_err(failed)?,
        })
    }

    #[staticmethod]
    fn from_json(text: &str) -> PyResult<Self> {
        let scene = CircuitScene::from_json(text).map_err(failed)?;
        Ok(Self {
            bench: scene.start().map_err(failed)?,
        })
    }

    fn to_json(&self) -> String {
        self.bench.scene.to_json()
    }

    /// The names of the components, wires included.
    fn components(&self) -> Vec<String> {
        self.bench
            .circuit
            .elements
            .iter()
            .map(|e| e.name.clone())
            .collect()
    }

    /// Seconds since the circuit was switched on.
    #[getter]
    fn time(&self) -> f64 {
        self.bench.state.time
    }

    /// True once a circuit followed in time has run its course.
    #[getter]
    fn finished(&self) -> bool {
        self.bench.finished()
    }

    /// Moves time on by this many seconds.
    fn advance(&mut self, seconds: f64) -> PyResult<()> {
        self.bench.advance(seconds).map_err(failed)
    }

    /// Volts across a component now, its `a` end above its `b` end.
    fn voltage(&self, name: &str) -> PyResult<f64> {
        Ok(self.bench.state.readings[self.index(name)?].voltage)
    }

    /// Amperes through a component now, from its `a` end to its `b` end.
    fn current(&self, name: &str) -> PyResult<f64> {
        Ok(self.bench.state.readings[self.index(name)?].current)
    }

    /// The complex amplitudes `(V, I)` of a component's alternating voltage and current; each quantity is
    /// the imaginary part of its amplitude times `exp(iωt)`.
    fn phasor<'py>(
        &self,
        py: Python<'py>,
        name: &str,
    ) -> PyResult<(Bound<'py, PyComplex>, Bound<'py, PyComplex>)> {
        let index = self.index(name)?;
        let phasors = self
            .bench
            .phasors
            .as_ref()
            .ok_or_else(|| PyValueError::new_err("this circuit is not an alternating one"))?;
        let complex = |z: num_complex::Complex64| PyComplex::from_doubles(py, z.re, z.im);
        Ok((
            complex(phasors[index].voltage),
            complex(phasors[index].current),
        ))
    }

    /// For a circuit followed in time: `(time, voltage, current)` of a component at every step so far.
    #[allow(clippy::type_complexity)]
    fn series<'py>(
        &self,
        py: Python<'py>,
        name: &str,
    ) -> PyResult<(
        Bound<'py, PyArray1<f64>>,
        Bound<'py, PyArray1<f64>>,
        Bound<'py, PyArray1<f64>>,
    )> {
        let index = self.index(name)?;
        let time: Vec<f64> = self.bench.history.iter().map(|s| s.time).collect();
        Ok((
            PyArray1::from_vec(py, time),
            PyArray1::from_vec(py, self.bench.series(index, Quantity::Voltage)),
            PyArray1::from_vec(py, self.bench.series(index, Quantity::Current)),
        ))
    }

    /// The textbook's answers: `{name: value}`.
    fn answers<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let out = PyDict::new(py);
        for (name, value, _) in self.bench.answers().map_err(failed)? {
            out.set_item(name, value)?;
        }
        Ok(out)
    }

    /// What the solved circuit gives for each answer it can be asked for: `{name: value}`.
    fn measured<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let out = PyDict::new(py);
        for (name, _, measured) in self.bench.answers().map_err(failed)? {
            if let Some(value) = measured {
                out.set_item(name, value)?;
            }
        }
        Ok(out)
    }
}
