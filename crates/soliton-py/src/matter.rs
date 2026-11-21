use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use soliton_matter::scenario::{catalogue, find, Params, Run};

/// The properties-of-matter catalogue as JSON: ids, titles and parameter schemas.
#[pyfunction]
pub fn matter_catalogue_json() -> String {
    let specs: Vec<_> = catalogue().iter().map(|s| s.spec).collect();
    serde_json::to_string(&specs).expect("catalogue serialises")
}

/// A properties-of-matter lesson (design 018), running.
#[pyclass(name = "MatterRun")]
pub struct MatterRun {
    run: Run,
}

fn as_python<'py, T: serde::Serialize>(py: Python<'py>, value: &T) -> PyResult<Bound<'py, PyAny>> {
    let text = serde_json::to_string(value).map_err(|e| PyValueError::new_err(e.to_string()))?;
    py.import("json")?.call_method1("loads", (text,))
}

#[pymethods]
impl MatterRun {
    /// A catalogue scenario with chosen parameters, e.g. `Run.scenario("buoyancy", block=917)`.
    #[staticmethod]
    #[pyo3(signature = (id, **params))]
    fn scenario(id: &str, params: Option<&Bound<'_, PyDict>>) -> PyResult<Self> {
        let scenario =
            find(id).ok_or_else(|| PyValueError::new_err(format!("unknown scenario `{id}`")))?;
        let mut chosen = Params::default();
        if let Some(params) = params {
            let known: Vec<&str> = scenario.spec.params.iter().map(|p| p.name).collect();
            for (key, value) in params.iter() {
                let name = key.extract::<String>()?;
                if !known.contains(&name.as_str()) {
                    return Err(PyValueError::new_err(format!(
                        "`{id}` has no parameter `{name}`; it has {known:?}"
                    )));
                }
                chosen = chosen.set(&name, value.extract::<f64>()?);
            }
        }
        Ok(Self {
            run: scenario.start(&chosen),
        })
    }

    /// Moves on by this many seconds of watching.
    fn advance(&mut self, seconds: f64) {
        self.run.advance(seconds);
    }

    /// True once a lesson that plays has run its course.
    #[getter]
    fn finished(&self) -> bool {
        self.run.finished()
    }

    /// True for a lesson that is a law evaluated, with nothing to play.
    #[getter]
    fn still(&self) -> bool {
        self.run.still()
    }

    /// The readings: `{label: (value, unit)}`, or `{label: words}` for one that is not a quantity.
    fn readings<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let out = PyDict::new(py);
        for r in self.run.readings() {
            if r.text.is_empty() {
                out.set_item(r.label, (r.value, r.unit))?;
            } else {
                out.set_item(r.label, r.text)?;
            }
        }
        Ok(out)
    }

    /// The graphs as plain lists and dicts.
    fn plots<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        as_python(py, &self.run.plots())
    }

    /// What the playground draws, as a dict with a `kind`.
    fn picture<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        as_python(py, &self.run.picture())
    }

    /// The textbook answers: `{name: value}`.
    fn answers<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let out = PyDict::new(py);
        for a in self.run.answers() {
            out.set_item(a.name, a.value)?;
        }
        Ok(out)
    }

    /// What the run itself has found so far: `{name: value}`.
    fn measured<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let out = PyDict::new(py);
        for a in self.run.answers() {
            if let Some(value) = a.measured {
                out.set_item(a.name, value)?;
            }
        }
        Ok(out)
    }
}
