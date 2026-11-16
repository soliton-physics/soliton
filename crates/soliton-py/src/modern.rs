use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use soliton_modern::scenario::{catalogue, find, Params, Run};

/// The modern physics catalogue as JSON: ids, titles and parameter schemas.
#[pyfunction]
pub fn modern_catalogue_json() -> String {
    let specs: Vec<_> = catalogue().iter().map(|s| s.spec).collect();
    serde_json::to_string(&specs).expect("catalogue serialises")
}

/// Wavelength in nm of an electron accelerated from rest through a voltage.
#[pyfunction]
pub fn electron_wavelength(voltage: f64) -> f64 {
    soliton_modern::matter::electron_wavelength(voltage)
}

/// Energy in eV of level `n` of a one-electron atom with nuclear charge `z`.
#[pyfunction]
#[pyo3(signature = (n, z=1.0))]
pub fn bohr_energy(n: u32, z: f64) -> f64 {
    soliton_modern::atom::energy(z, n)
}

/// Wavelength in nm of the photon given out when the electron falls from `upper` to `lower`.
#[pyfunction]
#[pyo3(signature = (upper, lower, z=1.0))]
pub fn line_wavelength(upper: u32, lower: u32, z: f64) -> PyResult<f64> {
    if lower == 0 || upper <= lower {
        return Err(PyValueError::new_err(
            "levels must satisfy upper > lower >= 1",
        ));
    }
    Ok(soliton_modern::atom::wavelength(z, upper, lower))
}

/// Angle in degrees an alpha of `energy` MeV is turned through by a nucleus of charge `z` at an impact
/// parameter in fm, found by integrating its path.
#[pyfunction]
pub fn alpha_deflection(energy: f64, z: f64, impact: f64) -> f64 {
    soliton_modern::rutherford::fly(energy, z, impact).turned
}

/// Binding energy in MeV of a nucleus of `a` nucleons and `z` protons, by the liquid-drop formula.
#[pyfunction]
pub fn liquid_drop(a: u32, z: u32) -> PyResult<f64> {
    if z == 0 || z > a {
        return Err(PyValueError::new_err("need 1 <= z <= a"));
    }
    Ok(soliton_modern::nucleus::liquid_drop(a, z))
}

/// A modern physics lesson (design 017), running.
#[pyclass(name = "ModernRun")]
pub struct ModernRun {
    run: Run,
}

fn as_python<'py, T: serde::Serialize>(py: Python<'py>, value: &T) -> PyResult<Bound<'py, PyAny>> {
    let text = serde_json::to_string(value).map_err(|e| PyValueError::new_err(e.to_string()))?;
    py.import("json")?.call_method1("loads", (text,))
}

#[pymethods]
impl ModernRun {
    /// A catalogue scenario with chosen parameters, e.g. `Run.scenario("bohr-model", upper=4)`.
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

    #[getter]
    fn finished(&self) -> bool {
        self.run.finished()
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

    /// The graphs as plain lists and dicts: `[{title, x_label, y_label, curves: [{name, x, y}]}]`.
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
