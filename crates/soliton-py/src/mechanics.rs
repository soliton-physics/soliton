use numpy::{IntoPyArray, PyArray1, PyArray2, PyArrayMethods};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use soliton_mechanics::scenario::{catalogue, find, Params};
use soliton_mechanics::scene::SceneFile;
use soliton_mechanics::{BodyId, EventKind, Recorder, Simulate, Simulation, Vec2};

/// The scenario catalogue as JSON: ids, titles, syllabus and parameter schemas.
#[pyfunction]
pub fn catalogue_json() -> String {
    let specs: Vec<_> = catalogue().iter().map(|s| s.spec).collect();
    serde_json::to_string(&specs).expect("catalogue serialises")
}

/// A scene as data (design 004): built from the catalogue or from JSON, run into a `Run`.
#[pyclass]
pub struct Scene {
    file: SceneFile,
}

#[pymethods]
impl Scene {
    /// A catalogue scenario with chosen parameters, e.g. `Scene.scenario("atwood", m1=2, m2=3)`.
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
            file: scenario.scene_file(&chosen),
        })
    }

    #[staticmethod]
    fn from_json(text: &str) -> PyResult<Self> {
        let file = SceneFile::from_json(text).map_err(|e| PyValueError::new_err(e.to_string()))?;
        file.build()
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self { file })
    }

    #[staticmethod]
    fn load(path: &str) -> PyResult<Self> {
        let text = std::fs::read_to_string(path)?;
        Self::from_json(&text)
    }

    fn to_json(&self) -> String {
        self.file.to_json()
    }

    /// The electric field `(Ex, Ey)` in N/C at a point of the scene as it starts: of every charge and of
    /// every uniform field there.
    fn electric_field(&self, x: f64, y: f64) -> PyResult<(f64, f64)> {
        let scene = self
            .file
            .build()
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        let e = scene.world.electric_field(Vec2::new(x, y), 0.0);
        Ok((e.x, e.y))
    }

    /// The electric potential in volts at a point of the scene as it starts, zero at infinity for charges.
    /// A field held between plates has no potential and adds nothing.
    fn electric_potential(&self, x: f64, y: f64) -> PyResult<f64> {
        let scene = self
            .file
            .build()
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(scene.world.electric_potential(Vec2::new(x, y)))
    }

    /// The electric flux in N·m²/C out of a sphere of `radius` centred at a point of the scene, summed over
    /// its surface. By Gauss's law it is the charge inside over ε₀.
    fn electric_flux(&self, x: f64, y: f64, radius: f64) -> PyResult<f64> {
        let scene = self
            .file
            .build()
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(scene.world.electric_flux(Vec2::new(x, y), radius))
    }

    /// The magnetic field `(Bx, By)` in tesla in the plane of the page at a point of the scene as it starts:
    /// of every bar magnet and of the background field.
    fn magnetic_field(&self, x: f64, y: f64) -> PyResult<(f64, f64)> {
        let scene = self
            .file
            .build()
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        let b = scene.world.magnetic_field_in_plane(Vec2::new(x, y));
        Ok((b.x, b.y))
    }

    fn save(&self, path: &str) -> PyResult<()> {
        std::fs::write(path, self.file.to_json())?;
        Ok(())
    }

    #[getter]
    fn title(&self) -> String {
        self.file.title.clone()
    }

    #[getter]
    fn dt(&self) -> f64 {
        self.file.world.dt
    }

    #[getter]
    fn labels(&self) -> Vec<String> {
        self.file.bodies.iter().map(|b| b.id.clone()).collect()
    }

    /// Textbook answers for this scene's parameters.
    fn answers<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        for a in &self.file.answers {
            dict.set_item(&a.name, a.value)?;
        }
        Ok(dict)
    }

    /// Runs the scene until `until` seconds, sampling every `record_every` steps.
    #[pyo3(signature = (until, record_every = 10))]
    fn run(&self, until: f64, record_every: u64) -> PyResult<Run> {
        let scene = self
            .file
            .build()
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        let labels = scene.labels.clone();
        let mut sim = Simulation::new(scene.world, scene.dt);
        sim.finish_when(scene.finish);
        let mut recorder = Recorder::every(record_every.max(1));
        sim.run_until(until, &mut recorder);
        let finished = sim.finished().map(|f| (f.time, f.describe(&labels)));
        let events = sim
            .events()
            .iter()
            .map(|e| {
                let (kind, bodies) = match e.kind {
                    EventKind::ContactBegin { a, b } => ("contact_begin", vec![a, b]),
                    EventKind::ContactEnd { a, b } => ("contact_end", vec![a, b]),
                    EventKind::RopeTaut { rope } => ("rope_taut", vec![BodyId::from_index(rope)]),
                    EventKind::RopeSlack { rope } => ("rope_slack", vec![BodyId::from_index(rope)]),
                    EventKind::FrictionSlip { a, b } => ("friction_slip", vec![a, b]),
                    EventKind::FrictionStick { a, b } => ("friction_stick", vec![a, b]),
                };
                let names = if kind.starts_with("rope") {
                    Vec::new()
                } else {
                    bodies.iter().map(|b| labels[b.index()].clone()).collect()
                };
                (e.time, kind.to_string(), names)
            })
            .collect();
        Ok(Run {
            recorder,
            labels,
            events,
            time_end: sim.time(),
            finished,
        })
    }
}

/// Recorded time series of one run, as NumPy arrays.
#[pyclass]
pub struct Run {
    recorder: Recorder,
    labels: Vec<String>,
    events: Vec<(f64, String, Vec<String>)>,
    time_end: f64,
    finished: Option<(f64, String)>,
}

impl Run {
    fn id(&self, label: &str) -> PyResult<BodyId> {
        self.labels
            .iter()
            .position(|l| l == label)
            .map(BodyId::from_index)
            .ok_or_else(|| {
                PyValueError::new_err(format!(
                    "no body named `{label}`; bodies are {:?}",
                    self.labels
                ))
            })
    }
}

fn pairs<'py>(
    py: Python<'py>,
    values: impl Iterator<Item = soliton_mechanics::Vec2>,
) -> PyResult<Bound<'py, PyArray2<f64>>> {
    let flat: Vec<f64> = values.flat_map(|v| [v.x, v.y]).collect();
    let n = flat.len() / 2;
    flat.into_pyarray(py).reshape([n, 2])
}

#[pymethods]
impl Run {
    #[getter]
    fn t<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        self.recorder.time.clone().into_pyarray(py)
    }

    #[getter]
    fn time_end(&self) -> f64 {
        self.time_end
    }

    /// `{"time", "reason"}` once the problem the scene poses has ended within the run, else `None`.
    #[getter]
    fn finished<'py>(&self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyDict>>> {
        let Some((time, reason)) = &self.finished else {
            return Ok(None);
        };
        let dict = PyDict::new(py);
        dict.set_item("time", time)?;
        dict.set_item("reason", reason)?;
        Ok(Some(dict))
    }

    #[getter]
    fn labels(&self) -> Vec<String> {
        self.labels.clone()
    }

    /// Kinematics of one body: `position`, `velocity`, `acceleration` as `(N, 2)` arrays and
    /// `angle`, `angular_velocity`, `angular_acceleration` as `(N,)` arrays.
    fn body<'py>(&self, py: Python<'py>, label: &str) -> PyResult<Bound<'py, PyDict>> {
        let record = self.recorder.body(self.id(label)?);
        let dict = PyDict::new(py);
        dict.set_item("position", pairs(py, record.position.iter().copied())?)?;
        dict.set_item("velocity", pairs(py, record.velocity.iter().copied())?)?;
        dict.set_item(
            "acceleration",
            pairs(py, record.acceleration.iter().copied())?,
        )?;
        dict.set_item("angle", record.angle.clone().into_pyarray(py))?;
        dict.set_item(
            "angular_velocity",
            record.angular_velocity.clone().into_pyarray(py),
        )?;
        dict.set_item(
            "angular_acceleration",
            record.angular_acceleration.clone().into_pyarray(py),
        )?;
        Ok(dict)
    }

    /// The net force of one kind (`gravity`, `normal`, `friction`, `tension`, `applied`, `spring`, `electric`, `magnetic`, ...) on a body,
    /// as an `(N, 2)` array.
    fn force<'py>(
        &self,
        py: Python<'py>,
        label: &str,
        kind: &str,
    ) -> PyResult<Bound<'py, PyArray2<f64>>> {
        let forces = self
            .recorder
            .force_named(self.id(label)?, kind)
            .ok_or_else(|| PyValueError::new_err(format!("unknown force kind `{kind}`")))?;
        pairs(py, forces.into_iter())
    }

    /// The energy ledger over time: `kinetic`, `potential`, `dissipated`, `external_work`, `conserved`.
    fn energy<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        let e = &self.recorder.energy;
        dict.set_item(
            "kinetic",
            e.iter()
                .map(|x| x.kinetic)
                .collect::<Vec<_>>()
                .into_pyarray(py),
        )?;
        dict.set_item(
            "potential",
            e.iter()
                .map(|x| x.potential)
                .collect::<Vec<_>>()
                .into_pyarray(py),
        )?;
        dict.set_item(
            "dissipated",
            e.iter()
                .map(|x| x.dissipated)
                .collect::<Vec<_>>()
                .into_pyarray(py),
        )?;
        dict.set_item(
            "external_work",
            e.iter()
                .map(|x| x.external_work)
                .collect::<Vec<_>>()
                .into_pyarray(py),
        )?;
        dict.set_item(
            "conserved",
            e.iter()
                .map(|x| x.conserved())
                .collect::<Vec<_>>()
                .into_pyarray(py),
        )?;
        Ok(dict)
    }

    /// Events as dicts with `time`, `kind` and `bodies`.
    #[getter]
    fn events<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        let list = PyList::empty(py);
        for (time, kind, bodies) in &self.events {
            let dict = PyDict::new(py);
            dict.set_item("time", time)?;
            dict.set_item("kind", kind)?;
            dict.set_item("bodies", bodies.clone())?;
            list.append(dict)?;
        }
        Ok(list)
    }
}
