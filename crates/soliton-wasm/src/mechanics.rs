use js_sys::Float64Array;
use serde::Serialize;
use soliton_mechanics::scenario::{catalogue, find, Params, Scene};
use soliton_mechanics::scene::SceneFile;
use soliton_mechanics::{
    BodyId, BodyKind, Finished, ForceLabel, Recorder, Shape, Simulate, Simulation, Sketch, Trigger,
    Vec2,
};
use wasm_bindgen::prelude::*;

/// Serialises to plain JavaScript objects and arrays, never `Map`s.
fn to_js<T: Serialize>(value: &T) -> Result<JsValue, JsError> {
    value
        .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .map_err(|e| JsError::new(&e.to_string()))
}

/// The scenario catalogue with parameter schemas, for building the picker and the sliders.
#[wasm_bindgen]
pub fn mechanics_catalogue() -> Result<JsValue, JsError> {
    let specs: Vec<soliton_mechanics::scenario::ScenarioSpec> =
        catalogue().iter().map(|s| s.spec).collect();
    to_js(&specs)
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum ShapeInfo {
    Circle { radius: f64 },
    Polygon { vertices: Vec<[f64; 2]> },
    Halfplane { normal: [f64; 2], offset: f64 },
}

#[derive(Serialize)]
struct BodyInfo {
    label: String,
    kind: &'static str,
    mass: f64,
    shape: ShapeInfo,
}

#[derive(Serialize)]
struct ForceInfo {
    label: &'static str,
    /// An impact impulse spread over one step rather than a steady force.
    impulsive: bool,
    force: [f64; 2],
    point: [f64; 2],
    torque: f64,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum SketchInfo {
    Line { points: [[f64; 2]; 2] },
    Polyline { points: Vec<[f64; 2]> },
    Circle { center: [f64; 2], radius: f64 },
    Spring { points: [[f64; 2]; 2] },
    Pivot { point: [f64; 2] },
}

#[derive(Serialize)]
struct EventInfo {
    time: f64,
    kind: String,
    bodies: Vec<usize>,
}

#[derive(Serialize)]
struct EnergyInfo {
    kinetic: f64,
    potential: f64,
    dissipated: f64,
    external_work: f64,
    conserved: f64,
}

#[derive(Serialize)]
struct GrabInfo {
    body: usize,
    point: [f64; 2],
    target: [f64; 2],
}

#[derive(Serialize)]
struct FinishInfo {
    time: f64,
    reason: String,
}

#[derive(Serialize)]
struct ExactInfo {
    name: String,
    value: f64,
}

fn v(p: soliton_mechanics::Vec2) -> [f64; 2] {
    [p.x, p.y]
}

/// A running mechanics scenario, built from the catalogue with chosen parameters.
#[wasm_bindgen]
pub struct MechanicsWorld {
    sim: Simulation,
    recorder: Recorder,
    labels: Vec<String>,
    focus: BodyId,
    exact: Vec<(String, f64)>,
    file: SceneFile,
    view: soliton_mechanics::scenario::View,
    drained: usize,
    announced: Option<Finished>,
}

#[wasm_bindgen]
impl MechanicsWorld {
    /// `params` is a plain object `{ name: value }`; missing entries take the scenario defaults.
    #[wasm_bindgen(constructor)]
    pub fn new(id: &str, params: JsValue, record_every: u32) -> Result<MechanicsWorld, JsError> {
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
        Self::from_file(scenario.scene_file(&p), record_every)
    }

    /// Builds a world from scene JSON (design 004).
    pub fn from_scene(json: &str, record_every: u32) -> Result<MechanicsWorld, JsError> {
        let file = SceneFile::from_json(json).map_err(|e| JsError::new(&e.to_string()))?;
        Self::from_file(file, record_every)
    }

    fn from_file(file: SceneFile, record_every: u32) -> Result<MechanicsWorld, JsError> {
        let Scene {
            world,
            dt,
            labels,
            focus,
            exact,
            view,
            finish,
        } = file.build().map_err(|e| JsError::new(&e.to_string()))?;
        let mut sim = Simulation::new(world, dt);
        sim.finish_when(finish);
        let mut recorder = Recorder::every(record_every.max(1) as u64);
        recorder.observe(&sim);
        Ok(Self {
            sim,
            recorder,
            labels,
            focus,
            exact,
            view,
            file,
            drained: 0,
            announced: None,
        })
    }

    /// The scene this world was built from, as JSON.
    pub fn scene_json(&self) -> String {
        self.file.to_json()
    }

    pub fn title(&self) -> String {
        self.file.title.clone()
    }

    /// Steps, stopping early the moment the problem ends so a front end can pause exactly there.
    /// Returns true when this call reached the end; later calls carry on past it, and stop once more if
    /// everything then comes to rest.
    pub fn step(&mut self, steps: usize) -> bool {
        for _ in 0..steps {
            self.sim.step();
            if self
                .sim
                .steps()
                .is_multiple_of(self.recorder.interval_public())
            {
                self.recorder.observe(&self.sim);
            }
            if self.sim.finished() != self.announced {
                self.announced = self.sim.finished();
                if self.recorder.time.last() != Some(&self.sim.time()) {
                    self.recorder.observe(&self.sim);
                }
                return true;
            }
        }
        false
    }

    /// `{ time, reason }` once the problem has ended, otherwise `null`.
    pub fn finished(&self) -> Result<JsValue, JsError> {
        match self.sim.finished() {
            Some(f) => to_js(&FinishInfo {
                time: f.time,
                reason: f.describe(&self.labels),
            }),
            None => Ok(JsValue::NULL),
        }
    }

    /// Takes hold of a body at a world point (design 008). From now until `release`, a spring pulls that point
    /// towards wherever `drag_to` last put the hand. Returns false for a body that cannot be moved.
    pub fn grab(&mut self, body: usize, x: f64, y: f64) -> bool {
        let point = Vec2::new(x, y);
        self.sim
            .world_mut()
            .grab(BodyId::from_index(body), point, point)
    }

    pub fn drag_to(&mut self, x: f64, y: f64) {
        self.sim.world_mut().move_grab(Vec2::new(x, y));
    }

    pub fn release(&mut self) {
        self.sim.world_mut().release();
    }

    /// The held body and where the hand is, as `{ body, point: [x, y], target: [x, y] }`, or `null`.
    pub fn grabbed(&self) -> Result<JsValue, JsError> {
        let world = self.sim.world();
        match world.grabbed() {
            Some(grab) => to_js(&GrabInfo {
                body: grab.body.index(),
                point: v(world.bodies().world_point(grab.body, grab.local)),
                target: v(grab.target),
            }),
            None => Ok(JsValue::NULL),
        }
    }

    /// The movable body under a world point, or the nearest one within `slop` metres.
    pub fn body_at(&self, x: f64, y: f64, slop: f64) -> Option<usize> {
        self.sim
            .world()
            .body_at(Vec2::new(x, y), slop)
            .map(BodyId::index)
    }

    /// The scene as it stands now, as JSON: current positions and velocities, lengths written out, times counted
    /// from this moment. Loading it carries on from here.
    pub fn scene_json_now(&self) -> Result<String, JsError> {
        self.file
            .snapshot(self.sim.world(), self.sim.time())
            .map(|file| file.to_json())
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// The problem's time window, when the scene has one.
    pub fn horizon(&self) -> Option<f64> {
        self.sim
            .finish_rules()
            .iter()
            .find_map(|rule| match rule.trigger {
                Trigger::Time(end) => Some(end + rule.hold),
                _ => None,
            })
    }

    pub fn time(&self) -> f64 {
        self.sim.time()
    }

    pub fn dt(&self) -> f64 {
        self.sim.dt()
    }

    pub fn focus(&self) -> usize {
        self.focus.index()
    }

    pub fn view(&self) -> Result<JsValue, JsError> {
        to_js(&self.view)
    }

    /// Static description of every body: label, kind, mass, shape.
    pub fn bodies(&self) -> Result<JsValue, JsError> {
        let b = self.sim.world().bodies();
        let infos: Vec<BodyInfo> = b
            .ids()
            .map(|id| BodyInfo {
                label: self.labels[id.index()].clone(),
                kind: match b.kind(id) {
                    BodyKind::Particle => "particle",
                    BodyKind::Rigid => "rigid",
                    BodyKind::Static => "static",
                },
                mass: b.mass(id),
                shape: match b.shape(id) {
                    Shape::Circle(c) => ShapeInfo::Circle { radius: c.radius },
                    Shape::Polygon(p) => ShapeInfo::Polygon {
                        vertices: p.vertices().iter().map(|&q| v(q)).collect(),
                    },
                    Shape::HalfPlane(h) => ShapeInfo::Halfplane {
                        normal: v(h.normal),
                        offset: h.offset,
                    },
                },
            })
            .collect();
        to_js(&infos)
    }

    /// `[x, y, angle, vx, vy, omega, ax, ay]` per body, flattened.
    pub fn state(&self) -> Float64Array {
        let world = self.sim.world();
        let b = world.bodies();
        let mut out = Vec::with_capacity(8 * b.len());
        for id in b.ids() {
            let (p, vel, a) = (b.position(id), b.velocity(id), world.acceleration(id));
            out.extend_from_slice(&[
                p.x,
                p.y,
                b.angle(id),
                vel.x,
                vel.y,
                b.angular_velocity(id),
                a.x,
                a.y,
            ]);
        }
        Float64Array::from(out.as_slice())
    }

    /// The free-body diagram of one body at the current state.
    pub fn forces(&self, body: usize) -> Result<JsValue, JsError> {
        let infos: Vec<ForceInfo> = self
            .sim
            .world()
            .forces_on(BodyId::from_index(body))
            .map(|s| ForceInfo {
                label: s.label.as_str(),
                impulsive: s.impulsive,
                force: v(s.force),
                point: v(s.point),
                torque: s.torque,
            })
            .collect();
        to_js(&infos)
    }

    /// Ropes, pulleys, rods, springs and pivots at the current state.
    pub fn sketches(&self) -> Result<JsValue, JsError> {
        let infos: Vec<SketchInfo> = self
            .sim
            .world()
            .sketches()
            .into_iter()
            .map(|s| match s {
                Sketch::Line(a, b) => SketchInfo::Line {
                    points: [v(a), v(b)],
                },
                Sketch::Polyline(points) => SketchInfo::Polyline {
                    points: points.into_iter().map(v).collect(),
                },
                Sketch::Circle { center, radius } => SketchInfo::Circle {
                    center: v(center),
                    radius,
                },
                Sketch::Spring(a, b) => SketchInfo::Spring {
                    points: [v(a), v(b)],
                },
                Sketch::Pivot(p) => SketchInfo::Pivot { point: v(p) },
            })
            .collect();
        to_js(&infos)
    }

    pub fn exact(&self) -> Result<JsValue, JsError> {
        let infos: Vec<ExactInfo> = self
            .exact
            .iter()
            .map(|(name, value)| ExactInfo {
                name: name.clone(),
                value: *value,
            })
            .collect();
        to_js(&infos)
    }

    pub fn energy(&self) -> Result<JsValue, JsError> {
        let e = self.sim.energy();
        let info = EnergyInfo {
            kinetic: e.kinetic,
            potential: e.potential,
            dissipated: e.dissipated,
            external_work: e.external_work,
            conserved: e.conserved(),
        };
        to_js(&info)
    }

    /// Events since the previous call.
    pub fn events(&mut self) -> Result<JsValue, JsError> {
        use soliton_mechanics::EventKind as K;
        let all = self.sim.events();
        let infos: Vec<EventInfo> = all[self.drained..]
            .iter()
            .map(|e| {
                let (kind, bodies) = match e.kind {
                    K::ContactBegin { a, b } => ("contact_begin", vec![a.index(), b.index()]),
                    K::ContactEnd { a, b } => ("contact_end", vec![a.index(), b.index()]),
                    K::RopeTaut { rope } => ("rope_taut", vec![rope]),
                    K::RopeSlack { rope } => ("rope_slack", vec![rope]),
                    K::FrictionSlip { a, b } => ("friction_slip", vec![a.index(), b.index()]),
                    K::FrictionStick { a, b } => ("friction_stick", vec![a.index(), b.index()]),
                };
                EventInfo {
                    time: e.time,
                    kind: kind.to_string(),
                    bodies,
                }
            })
            .collect();
        self.drained = all.len();
        to_js(&infos)
    }

    pub fn series_time(&self) -> Float64Array {
        Float64Array::from(self.recorder.time.as_slice())
    }

    /// A recorded quantity of one body: `x`, `y`, `vx`, `vy`, `speed`, `ax`, `ay`, `angle`, `omega`.
    pub fn series(&self, body: usize, quantity: &str) -> Result<Float64Array, JsError> {
        let r = self.recorder.body(BodyId::from_index(body));
        let values: Vec<f64> = match quantity {
            "x" => r.position.iter().map(|p| p.x).collect(),
            "y" => r.position.iter().map(|p| p.y).collect(),
            "vx" => r.velocity.iter().map(|p| p.x).collect(),
            "vy" => r.velocity.iter().map(|p| p.y).collect(),
            "speed" => r.velocity.iter().map(|p| p.length()).collect(),
            "ax" => r.acceleration.iter().map(|p| p.x).collect(),
            "ay" => r.acceleration.iter().map(|p| p.y).collect(),
            "angle" => r.angle.clone(),
            "omega" => r.angular_velocity.clone(),
            other => return Err(JsError::new(&format!("unknown quantity `{other}`"))),
        };
        Ok(Float64Array::from(values.as_slice()))
    }

    /// The recorded net force with `label` on one body, as `[fx, fy]` pairs flattened.
    pub fn series_force(&self, body: usize, label: &str) -> Result<Float64Array, JsError> {
        let label = [
            ForceLabel::Gravity,
            ForceLabel::Applied,
            ForceLabel::Torque,
            ForceLabel::Spring,
            ForceLabel::Drag,
            ForceLabel::Driving,
            ForceLabel::Tension,
            ForceLabel::Normal,
            ForceLabel::Friction,
            ForceLabel::Reaction,
        ]
        .into_iter()
        .find(|l| l.as_str() == label)
        .ok_or_else(|| JsError::new(&format!("unknown force label `{label}`")))?;
        let values: Vec<f64> = self
            .recorder
            .force(BodyId::from_index(body), label)
            .iter()
            .flat_map(|f| [f.x, f.y])
            .collect();
        Ok(Float64Array::from(values.as_slice()))
    }

    /// Recorded energy ledger: `kinetic`, `potential`, `dissipated`, `external_work`, `conserved`.
    pub fn series_energy(&self, quantity: &str) -> Result<Float64Array, JsError> {
        let values: Vec<f64> = match quantity {
            "kinetic" => self.recorder.energy.iter().map(|e| e.kinetic).collect(),
            "potential" => self.recorder.energy.iter().map(|e| e.potential).collect(),
            "dissipated" => self.recorder.energy.iter().map(|e| e.dissipated).collect(),
            "external_work" => self
                .recorder
                .energy
                .iter()
                .map(|e| e.external_work)
                .collect(),
            "conserved" => self.recorder.energy.iter().map(|e| e.conserved()).collect(),
            other => return Err(JsError::new(&format!("unknown energy quantity `{other}`"))),
        };
        Ok(Float64Array::from(values.as_slice()))
    }
}
