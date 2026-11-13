//! The ray optics bench as the playground sees it (design 013): a scene traced once, and everything to draw.

use serde::Serialize;
use soliton_optics::scenario::{catalogue, find, Params};
use soliton_optics::scene::{OpticsScene, Traces};
use soliton_optics::{Element, End, EventKind, Vec2};
use wasm_bindgen::prelude::*;

fn to_js<T: Serialize>(value: &T) -> Result<JsValue, JsError> {
    value
        .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .map_err(|e| JsError::new(&e.to_string()))
}

fn v(p: Vec2) -> [f64; 2] {
    [p.x, p.y]
}

/// The optics scenarios with their parameter schemas.
#[wasm_bindgen]
pub fn optics_catalogue() -> Result<JsValue, JsError> {
    let specs: Vec<soliton_optics::scenario::ScenarioSpec> =
        catalogue().iter().map(|s| s.spec).collect();
    to_js(&specs)
}

/// Something on the bench, as a line to draw. Glass is a closed outline; a lens is the line across it.
#[derive(Serialize)]
struct ElementInfo {
    kind: &'static str,
    points: Vec<[f64; 2]>,
    /// Refractive index of glass; zero for anything else.
    index: f64,
    /// Focal length of an ideal lens, m; zero for anything else.
    focal: f64,
}

#[derive(Serialize)]
struct EventInfo {
    at: [f64; 2],
    kind: &'static str,
    /// Unit normal on the side the ray came from.
    normal: [f64; 2],
    /// Degrees from the normal, arriving and leaving.
    incidence: f64,
    emergence: f64,
}

#[derive(Serialize)]
struct RayInfo {
    points: Vec<[f64; 2]>,
    events: Vec<EventInfo>,
    /// `escaped`, `absorbed`, or `exhausted` for a ray still bouncing when it was given up on.
    end: &'static str,
    /// Mark the normal and the angles where this ray meets a surface.
    angles: bool,
    /// Draw it lightly: it is background to another ray.
    faint: bool,
}

#[derive(Serialize)]
struct ImageInfo {
    base: [f64; 2],
    tip: [f64; 2],
    real: bool,
}

#[derive(Serialize)]
struct ObjectInfo {
    base: [f64; 2],
    tip: [f64; 2],
    /// `null` when the image is at infinity.
    image: Option<ImageInfo>,
}

#[derive(Serialize)]
struct AnswerInfo {
    name: String,
    value: f64,
    /// What the traced rays give, where they can give it.
    measured: Option<f64>,
}

/// How finely a curved surface is drawn.
const ARC_STEPS: usize = 48;

#[wasm_bindgen]
pub struct OpticsBench {
    scene: OpticsScene,
    traces: Traces,
}

#[wasm_bindgen]
impl OpticsBench {
    /// `params` is a plain object `{ name: value }`; missing entries take the scenario defaults.
    #[wasm_bindgen(constructor)]
    pub fn new(id: &str, params: JsValue) -> Result<OpticsBench, JsError> {
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
        Self::from_file(scenario.scene(&p))
    }

    pub fn from_scene(json: &str) -> Result<OpticsBench, JsError> {
        let scene = OpticsScene::from_json(json).map_err(|e| JsError::new(&e.to_string()))?;
        Self::from_file(scene)
    }

    fn from_file(scene: OpticsScene) -> Result<OpticsBench, JsError> {
        let traces = scene.trace().map_err(|e| JsError::new(&e.to_string()))?;
        Ok(Self { scene, traces })
    }

    pub fn title(&self) -> String {
        self.scene.title.clone()
    }

    pub fn scene_json(&self) -> String {
        self.scene.to_json()
    }

    /// `{ center, half_width, half_height }`, or `undefined` for a scene that suggests none.
    pub fn view(&self) -> Result<JsValue, JsError> {
        to_js(&self.scene.view)
    }

    pub fn elements(&self) -> Result<JsValue, JsError> {
        let infos: Vec<ElementInfo> = self
            .traces
            .bench
            .elements
            .iter()
            .map(|element| match element {
                Element::Mirror(curve) | Element::Screen(curve) => ElementInfo {
                    kind: if matches!(element, Element::Mirror(_)) {
                        "mirror"
                    } else {
                        "screen"
                    },
                    points: curve.points(ARC_STEPS).into_iter().map(v).collect(),
                    index: 0.0,
                    focal: 0.0,
                },
                Element::Glass { outline, index } => {
                    // The curves of an outline are joined end to end in whichever order closes it.
                    let mut points: Vec<Vec2> = Vec::new();
                    for curve in outline {
                        let mut piece = curve.points(ARC_STEPS);
                        if let Some(&last) = points.last() {
                            if (piece[0] - last).length() > (piece[piece.len() - 1] - last).length()
                            {
                                piece.reverse();
                            }
                        }
                        points.extend(piece);
                    }
                    ElementInfo {
                        kind: "glass",
                        points: points.into_iter().map(v).collect(),
                        index: *index,
                        focal: 0.0,
                    }
                }
                Element::ThinLens {
                    centre,
                    axis,
                    focal,
                    aperture,
                } => {
                    let reach = axis.perp() * *aperture;
                    ElementInfo {
                        kind: "lens",
                        points: vec![v(*centre - reach), v(*centre + reach)],
                        index: 0.0,
                        focal: *focal,
                    }
                }
            })
            .collect();
        to_js(&infos)
    }

    pub fn rays(&self) -> Result<JsValue, JsError> {
        let infos: Vec<RayInfo> = self
            .traces
            .rays
            .iter()
            .map(|traced| RayInfo {
                points: traced.path.points.iter().map(|&p| v(p)).collect(),
                events: traced
                    .path
                    .events
                    .iter()
                    .map(|e| EventInfo {
                        at: v(e.at),
                        kind: match e.kind {
                            EventKind::Reflection => "reflection",
                            EventKind::Refraction => "refraction",
                            EventKind::TotalReflection => "total_reflection",
                            EventKind::Lens => "lens",
                            EventKind::Absorbed => "absorbed",
                        },
                        normal: v(e.normal),
                        incidence: e.incidence.to_degrees(),
                        emergence: e.emergence.to_degrees(),
                    })
                    .collect(),
                end: match traced.path.end {
                    End::Escaped => "escaped",
                    End::Absorbed => "absorbed",
                    End::Exhausted => "exhausted",
                },
                angles: traced.angles,
                faint: traced.faint,
            })
            .collect();
        to_js(&infos)
    }

    /// Each object with its paraxial image.
    pub fn objects(&self) -> Result<JsValue, JsError> {
        let infos: Vec<ObjectInfo> = self
            .scene
            .objects
            .iter()
            .zip(&self.traces.images)
            .map(|(object, image)| ObjectInfo {
                base: object.base,
                tip: object.tip,
                image: image.map(|i| ImageInfo {
                    base: v(i.base.at),
                    tip: v(i.tip.at),
                    real: i.base.real,
                }),
            })
            .collect();
        to_js(&infos)
    }

    /// Named points: `[{ at, label }]`.
    pub fn marks(&self) -> Result<JsValue, JsError> {
        to_js(&self.scene.marks)
    }

    /// Two points on the principal axis, or `undefined`.
    pub fn axis(&self) -> Result<JsValue, JsError> {
        to_js(&self.scene.axis)
    }

    /// `[{ name, value, measured }]`: the textbook's answer and what the traced rays give.
    pub fn answers(&self) -> Result<JsValue, JsError> {
        let infos: Vec<AnswerInfo> = self
            .traces
            .answers
            .iter()
            .map(|(name, value, measured)| AnswerInfo {
                name: name.clone(),
                value: *value,
                measured: *measured,
            })
            .collect();
        to_js(&infos)
    }

    /// The refractive index at a point.
    pub fn index_at(&self, x: f64, y: f64) -> f64 {
        self.traces.bench.index_at(Vec2::new(x, y))
    }
}
