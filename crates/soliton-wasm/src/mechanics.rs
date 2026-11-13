use js_sys::Float64Array;
use serde::Serialize;
use soliton_mechanics::scenario::{catalogue, find, Params, Scene};
use soliton_mechanics::scene::SceneFile;
use soliton_mechanics::{
    BodyId, BodyKind, Distribution, FieldKind, Finished, Recorder, Shape, Simulate, Simulation,
    Sketch, Trigger, Vec2,
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
    /// Electric charge in coulombs; zero for a body with none.
    charge: f64,
    /// `[moment, length]` of a bar magnet, A·m² and m, north towards the body's x axis; `null` for anything else.
    magnet: Option<[f64; 2]>,
    /// Moment taken on per tesla, A·m²/T: positive for a paramagnet, negative for a diamagnet, zero for most things.
    polarisability: f64,
    shape: ShapeInfo,
}

/// A fixed wire, sheet or shell of charge. `at` is where the wire meets the page, a point on the sheet, or the
/// centre of the shell; `amount` is C/m, C/m² or C.
#[derive(Serialize)]
struct DistributionInfo {
    kind: &'static str,
    at: [f64; 2],
    normal: [f64; 2],
    radius: f64,
    amount: f64,
}

/// A uniform field: electric in N/C as it is at this instant, or magnetic in tesla out of the page.
#[derive(Serialize)]
struct FieldInfo {
    kind: &'static str,
    electric: [f64; 2],
    magnetic: f64,
    /// `[min, max]` corners of the rectangle it is confined to, or `null` when it fills the plane.
    region: Option<[[f64; 2]; 2]>,
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
            playback: _,
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

    /// Seconds of the simulation to show per second of watching; one unless the scene says otherwise.
    pub fn playback(&self) -> f64 {
        self.file.playback.unwrap_or(1.0)
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
                charge: b.charge(id),
                magnet: b.magnet(id).map(|m| [m.moment, m.length]),
                polarisability: b.polarisability(id),
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

    /// The uniform electric and magnetic fields in the scene, for drawing.
    pub fn fields(&self) -> Result<JsValue, JsError> {
        let infos: Vec<FieldInfo> = self
            .sim
            .world()
            .fields(self.sim.time())
            .into_iter()
            .map(|f| {
                let (kind, electric, magnetic) = match f.kind {
                    FieldKind::Electric(e) => ("electric", v(e), 0.0),
                    FieldKind::Magnetic(b) => ("magnetic", [0.0, 0.0], b),
                };
                FieldInfo {
                    kind,
                    electric,
                    magnetic,
                    region: f.region.map(|r| [v(r.min), v(r.max)]),
                }
            })
            .collect();
        to_js(&infos)
    }

    /// `[Ex, Ey, V, Bz, Bx, By]` at a point: electric field in N/C, potential in volts, magnetic field out of
    /// the page and then in the plane, in tesla. The potential leaves out any field that has none (one held
    /// between plates, or alternating).
    pub fn field_at(&self, x: f64, y: f64) -> Float64Array {
        let world = self.sim.world();
        let point = Vec2::new(x, y);
        self.field_values(point, world.magnetic_field_in_plane(point))
    }

    /// The same at the centre of a body, leaving out the field of the body's own magnet: what a compass or a
    /// sample placed there is in.
    pub fn field_on(&self, body: usize) -> Float64Array {
        let world = self.sim.world();
        let id = BodyId::from_index(body);
        self.field_values(world.bodies().position(id), world.magnetic_field_on(id))
    }

    fn field_values(&self, point: Vec2, in_plane: Vec2) -> Float64Array {
        let world = self.sim.world();
        let e = world.electric_field(point, self.sim.time());
        let values = [
            e.x,
            e.y,
            world.electric_potential(point),
            world.magnetic_field(point),
            in_plane.x,
            in_plane.y,
        ];
        Float64Array::from(values.as_slice())
    }

    /// `[Bx, By]` in tesla at a point, of the fixed magnets and the background alone: the field a compass put
    /// there would line up with.
    pub fn fixed_field_at(&self, x: f64, y: f64) -> Float64Array {
        let b = self.sim.world().fixed_magnetic_field(Vec2::new(x, y));
        Float64Array::from([b.x, b.y].as_slice())
    }

    /// The electric flux out of a sphere centred at `(cx, cy)` in the page, N·m²/C, summed over its surface.
    pub fn flux_through(&self, cx: f64, cy: f64, radius: f64) -> f64 {
        self.sim.world().electric_flux(Vec2::new(cx, cy), radius)
    }

    /// The fixed wires, sheets and shells of charge in the scene, for drawing.
    pub fn distributions(&self) -> Result<JsValue, JsError> {
        let infos: Vec<DistributionInfo> = self
            .sim
            .world()
            .distributions()
            .into_iter()
            .map(|d| match d {
                Distribution::Wire { at, density } => DistributionInfo {
                    kind: "wire",
                    at: v(at),
                    normal: [0.0, 0.0],
                    radius: 0.0,
                    amount: density,
                },
                Distribution::Sheet {
                    point,
                    normal,
                    density,
                } => DistributionInfo {
                    kind: "sheet",
                    at: v(point),
                    normal: v(normal),
                    radius: 0.0,
                    amount: density,
                },
                Distribution::Shell {
                    centre,
                    radius,
                    charge,
                } => DistributionInfo {
                    kind: "shell",
                    at: v(centre),
                    normal: [0.0, 0.0],
                    radius,
                    amount: charge,
                },
            })
            .collect();
        to_js(&infos)
    }

    /// Field lines of the charges, `per_charge` from the largest, followed until they end on a charge or run
    /// `reach` from `(cx, cy)`. Points as `x, y` pairs, each line closed by a pair of `NaN`s.
    pub fn field_lines(&self, per_charge: usize, cx: f64, cy: f64, reach: f64) -> Float64Array {
        flatten(
            self.sim
                .world()
                .field_lines(per_charge, Vec2::new(cx, cy), reach, self.sim.time()),
        )
    }

    /// Lines of the magnetic field of the fixed bar magnets, from north poles round to south, in the same form.
    pub fn magnetic_field_lines(
        &self,
        per_pole: usize,
        cx: f64,
        cy: f64,
        reach: f64,
    ) -> Float64Array {
        flatten(
            self.sim
                .world()
                .magnetic_field_lines(per_pole, Vec2::new(cx, cy), reach),
        )
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
        let values: Vec<f64> = self
            .recorder
            .force_named(BodyId::from_index(body), label)
            .ok_or_else(|| JsError::new(&format!("unknown force label `{label}`")))?
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

/// Lines as `x, y` pairs, each closed by a pair of `NaN`s.
fn flatten(lines: Vec<Vec<Vec2>>) -> Float64Array {
    let mut out = Vec::new();
    for line in lines {
        out.extend(line.into_iter().flat_map(|p| [p.x, p.y]));
        out.extend([f64::NAN, f64::NAN]);
    }
    Float64Array::from(out.as_slice())
}
