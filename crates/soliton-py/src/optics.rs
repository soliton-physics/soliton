use numpy::{PyArray2, PyArrayMethods};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use soliton_optics::scenario::{catalogue, find, Params};
use soliton_optics::scene::{OpticsScene, Traces};

/// The optics catalogue as JSON: ids, titles, syllabus and parameter schemas.
#[pyfunction]
pub fn optics_catalogue_json() -> String {
    let specs: Vec<_> = catalogue().iter().map(|s| s.spec).collect();
    serde_json::to_string(&specs).expect("catalogue serialises")
}

/// An optical bench (design 013): a scene from the catalogue or from JSON, with its rays traced.
#[pyclass]
pub struct Bench {
    scene: OpticsScene,
    traces: Traces,
}

impl Bench {
    fn build(scene: OpticsScene) -> PyResult<Self> {
        let traces = scene
            .trace()
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self { scene, traces })
    }
}

#[pymethods]
impl Bench {
    /// A catalogue scenario with chosen parameters, e.g. `Bench.scenario("convex-lens", distance=25)`.
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
            OpticsScene::from_json(text).map_err(|e| PyValueError::new_err(e.to_string()))?;
        Self::build(scene)
    }

    fn to_json(&self) -> String {
        self.scene.to_json()
    }

    /// The textbook's answers: `{name: value}`.
    fn answers<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let out = PyDict::new(py);
        for (name, value, _) in &self.traces.answers {
            out.set_item(name, value)?;
        }
        Ok(out)
    }

    /// What the traced rays give for each answer they can be asked for: `{name: value}`.
    fn measured<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let out = PyDict::new(py);
        for (name, _, measured) in &self.traces.answers {
            if let Some(value) = measured {
                out.set_item(name, value)?;
            }
        }
        Ok(out)
    }

    /// Every ray as an `(N, 2)` array of the points it passes through, in metres.
    fn rays<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        let out = PyList::empty(py);
        for traced in &self.traces.rays {
            let flat: Vec<f64> = traced.path.points.iter().flat_map(|p| [p.x, p.y]).collect();
            let points =
                numpy::PyArray1::from_vec(py, flat).reshape([traced.path.points.len(), 2])?;
            out.append::<Bound<'py, PyArray2<f64>>>(points)?;
        }
        Ok(out)
    }

    /// The paraxial image of each object: `{"base": (x, y), "tip": (x, y), "real": bool}`, or `None` for one
    /// at infinity.
    fn images<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        let out = PyList::empty(py);
        for image in &self.traces.images {
            match image {
                Some(image) => {
                    let entry = PyDict::new(py);
                    entry.set_item("base", (image.base.at.x, image.base.at.y))?;
                    entry.set_item("tip", (image.tip.at.x, image.tip.at.y))?;
                    entry.set_item("real", image.base.real)?;
                    out.append(entry)?;
                }
                None => out.append(py.None())?,
            }
        }
        Ok(out)
    }
}
