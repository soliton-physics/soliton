//! The scenario catalogue (design 004): named, parameterised scenes in NCERT chapter order, each with its textbook
//! answers. Every scenario is produced as a [`SceneFile`], so the JSON a teacher can export is exactly what runs.

use crate::body::BodyId;
use crate::exact;
use crate::finish::Finish;
use crate::scene::{
    AnchorSpec, BodySpec, ConstraintSpec, FinishSpec, ForceSpec, RopeNodeSpec, SceneFile,
    ShapeSpec, SourceSpec, WrapSpec,
};
use crate::world::World;
use serde::{Deserialize, Serialize};
use soliton_core::Vec2;

const G: f64 = 9.8;

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Param {
    pub name: &'static str,
    pub label: &'static str,
    pub unit: &'static str,
    pub min: f64,
    pub max: f64,
    pub step: f64,
    pub default: f64,
}

const fn param(
    name: &'static str,
    label: &'static str,
    unit: &'static str,
    range: (f64, f64),
    step: f64,
    default: f64,
) -> Param {
    Param {
        name,
        label,
        unit,
        min: range.0,
        max: range.1,
        step,
        default,
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Syllabus {
    pub class: u8,
    pub chapter: &'static str,
    pub topic: &'static str,
}

/// What a scenario is, independent of any parameter values.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct ScenarioSpec {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub syllabus: Syllabus,
    pub params: &'static [Param],
    /// Suggested fixed time step.
    pub dt: f64,
    /// Suggested run length for graphs and the notebook.
    pub duration: f64,
}

/// Chosen parameter values; anything not set takes the spec's default.
#[derive(Clone, Debug, Default)]
pub struct Params {
    values: Vec<(String, f64)>,
}

impl Params {
    pub fn set(mut self, name: &str, value: f64) -> Self {
        self.values.retain(|(n, _)| n != name);
        self.values.push((name.to_string(), value));
        self
    }

    fn get(&self, spec: &ScenarioSpec, name: &str) -> f64 {
        self.values
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| *v)
            .or_else(|| {
                spec.params
                    .iter()
                    .find(|p| p.name == name)
                    .map(|p| p.default)
            })
            .unwrap_or_else(|| panic!("scenario `{}` has no parameter `{name}`", spec.id))
    }
}

/// A camera suggestion: what the playground should frame.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct View {
    pub center: [f64; 2],
    pub half_height: f64,
    /// When set, the frame is widened or zoomed out so this much also fits horizontally.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub half_width: Option<f64>,
}

/// A built scene: the live world plus everything the front ends need to present it.
pub struct Scene {
    pub world: World,
    pub dt: f64,
    pub labels: Vec<String>,
    /// The body the free-body diagram and graphs show by default.
    pub focus: BodyId,
    pub exact: Vec<(String, f64)>,
    pub view: View,
    pub finish: Vec<Finish>,
    /// Seconds of the simulation to show per second of watching.
    pub playback: f64,
}

impl std::fmt::Debug for Scene {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Scene")
            .field("bodies", &self.labels)
            .field("focus", &self.focus)
            .finish()
    }
}

pub struct Scenario {
    pub spec: ScenarioSpec,
    describe: fn(&ScenarioSpec, &Params) -> SceneFile,
}

impl Scenario {
    /// The scene as data, with the textbook answers for these parameters.
    pub fn scene_file(&self, params: &Params) -> SceneFile {
        let spec = &self.spec;
        (self.describe)(spec, params)
            .describe(spec.description)
            .syllabus(
                spec.syllabus.class,
                spec.syllabus.chapter,
                spec.syllabus.topic,
            )
    }

    pub fn build(&self, params: &Params) -> Scene {
        self.scene_file(params)
            .build()
            .unwrap_or_else(|e| panic!("scenario `{}`: {e}", self.spec.id))
    }
}

pub fn catalogue() -> &'static [Scenario] {
    CATALOGUE
}

pub fn find(id: &str) -> Option<&'static Scenario> {
    CATALOGUE.iter().find(|s| s.spec.id == id)
}

/// Every scenario ends at its suggested duration unless something more telling happens first.
fn scene(spec: &ScenarioSpec) -> SceneFile {
    SceneFile::new(spec.title, [0.0, -G], spec.dt).finish(FinishSpec::time(spec.duration))
}

/// A launched ball starts a hair above the ground so it is already free: no normal force, no contact to report.
const LAUNCH_GAP: f64 = 2e-3;

fn floor(friction: (f64, f64)) -> BodySpec {
    BodySpec::fixed(
        "ground",
        ShapeSpec::Halfplane {
            normal: [0.0, 1.0],
            offset: 0.0,
        },
    )
    .friction(friction.0, friction.1)
}

/// Outward normal of the sloped face and the unit vector pointing down the slope.
fn incline_surface(theta: f64) -> (Vec2, Vec2) {
    (
        Vec2::new(theta.sin(), theta.cos()),
        Vec2::new(theta.cos(), -theta.sin()),
    )
}

/// Fixed triangular incline rising to the left: top at `(0, L tan θ)`, foot at `(L, 0)`.
fn incline(theta: f64, length: f64, friction: (f64, f64)) -> BodySpec {
    let height = length * theta.tan();
    BodySpec::fixed(
        "incline",
        ShapeSpec::Polygon(vec![[0.0, 0.0], [length, 0.0], [0.0, height]]),
    )
    .friction(friction.0, friction.1)
}

/// Where a body of half-thickness `half` rests on the incline, `along` metres down from the top.
fn on_incline(theta: f64, length: f64, along: f64, half: f64) -> Vec2 {
    let (normal, down) = incline_surface(theta);
    Vec2::new(0.0, length * theta.tan()) + down * along + normal * half
}

fn ball(id: &str, mass: f64, radius: f64) -> BodySpec {
    BodySpec::particle(id, mass).shape(ShapeSpec::Circle(radius))
}

// ---------------------------------------------------------------- Motion in a straight line

static FREE_FALL: [Param; 1] = [param("height", "Drop height", "m", (1.0, 60.0), 0.5, 20.0)];

fn free_fall(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let h = p.get(spec, "height");
    scene(spec)
        .body(floor((1.0, 1.0)))
        .body(ball("ball", 1.0, 0.3).at(0.0, h + 0.3).restitution(0.0))
        .focus("ball")
        .finish(FinishSpec::contact("ball", "ground"))
        .answer("Time to reach the ground (s)", (2.0 * h / G).sqrt())
        .answer("Speed on impact (m/s)", (2.0 * G * h).sqrt())
        .view([0.0, 0.5 * h], 0.6 * h + 1.0)
}

static THROWN_UP: [Param; 1] = [param(
    "speed",
    "Launch speed",
    "m/s",
    (1.0, 40.0),
    0.5,
    15.0,
)];

fn thrown_upward(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let u = p.get(spec, "speed");
    let apex = u * u / (2.0 * G);
    scene(spec)
        .body(floor((1.0, 1.0)))
        .body(
            ball("ball", 1.0, 0.3)
                .at(0.0, 0.3 + LAUNCH_GAP)
                .velocity(0.0, u)
                .restitution(0.0),
        )
        .focus("ball")
        .finish(FinishSpec::contact("ball", "ground"))
        .answer("Maximum height (m)", apex)
        .answer("Time to the top (s)", u / G)
        .answer("Time of flight (s)", 2.0 * u / G)
        .view([0.0, 0.5 * apex], 0.6 * apex + 1.0)
}

static STOPPING: [Param; 2] = [
    param(
        "speed",
        "Speed before braking",
        "m/s",
        (1.0, 30.0),
        0.5,
        10.0,
    ),
    param(
        "friction",
        "Kinetic friction μk",
        "",
        (0.05, 1.0),
        0.01,
        0.5,
    ),
];

fn stopping_distance(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let (u, mu) = (p.get(spec, "speed"), p.get(spec, "friction"));
    let distance = exact::stopping_distance(u, mu, G);
    let time = u / (mu * G);
    SceneFile::new(spec.title, [0.0, -G], spec.dt)
        .finish(FinishSpec::time(1.5 * time + 1.0))
        .finish(FinishSpec::rest(0.5))
        .body(floor((mu, mu)))
        .body(
            BodySpec::rigid("block", 2.0, ShapeSpec::Rectangle([0.8, 0.5]))
                .at(0.0, 0.25)
                .velocity(u, 0.0)
                .restitution(0.0),
        )
        .focus("block")
        .answer("Stopping distance (m)", distance)
        .answer("Time to stop (s)", time)
        .answer("Deceleration (m/s²)", mu * G)
        .frame([0.5 * distance, 1.0], 0.5 * distance + 1.5, 2.5)
}

// ---------------------------------------------------------------- Motion in a plane

static PROJECTILE: [Param; 2] = [
    param("speed", "Launch speed", "m/s", (2.0, 40.0), 0.5, 20.0),
    param("angle", "Launch angle", "°", (5.0, 85.0), 1.0, 45.0),
];

fn projectile(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let (u, angle) = (p.get(spec, "speed"), p.get(spec, "angle").to_radians());
    let range = exact::projectile_range(u, angle, G);
    let height = exact::projectile_max_height(u, angle, G);
    scene(spec)
        .body(floor((1.0, 1.0)))
        .body(
            ball("ball", 1.0, 0.25)
                .at(0.0, 0.25 + LAUNCH_GAP)
                .velocity(u * angle.cos(), u * angle.sin())
                .restitution(0.0),
        )
        .focus("ball")
        .finish(FinishSpec::contact("ball", "ground"))
        .answer("Range (m)", range)
        .answer("Maximum height (m)", height)
        .answer(
            "Time of flight (s)",
            exact::projectile_flight_time(u, angle, G),
        )
        .frame(
            [0.5 * range, 0.5 * height],
            0.5 * range + 1.0,
            0.5 * height + 1.0,
        )
}

static PROJECTILE_HEIGHT: [Param; 3] = [
    param("speed", "Launch speed", "m/s", (2.0, 40.0), 0.5, 15.0),
    param("angle", "Launch angle", "°", (-60.0, 85.0), 1.0, 30.0),
    param("height", "Launch height", "m", (0.0, 50.0), 0.5, 10.0),
];

fn projectile_from_height(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let (u, angle, h) = (
        p.get(spec, "speed"),
        p.get(spec, "angle").to_radians(),
        p.get(spec, "height"),
    );
    let (ux, uy) = (u * angle.cos(), u * angle.sin());
    let flight = (uy + (uy * uy + 2.0 * G * h).sqrt()) / G;
    let range = ux * flight;
    scene(spec)
        .body(floor((1.0, 1.0)))
        .body(BodySpec::fixed(
            "cliff",
            ShapeSpec::Polygon(vec![[-2.0, 0.0], [0.0, 0.0], [0.0, h], [-2.0, h]]),
        ))
        .body(
            ball("ball", 1.0, 0.25)
                .at(0.0, h + 0.25 + LAUNCH_GAP)
                .velocity(ux, uy)
                .restitution(0.0),
        )
        .focus("ball")
        .finish(FinishSpec::contact("ball", "ground"))
        .answer("Horizontal range (m)", range)
        .answer("Time of flight (s)", flight)
        .answer("Speed on landing (m/s)", (u * u + 2.0 * G * h).sqrt())
        .frame(
            [0.5 * range, 0.5 * (h + apex_above(uy, h))],
            0.5 * range + 2.5,
            0.5 * (h + apex_above(uy, h)) + 1.0,
        )
}

/// Highest point of a flight launched from height `h` with vertical speed `uy`.
fn apex_above(uy: f64, h: f64) -> f64 {
    h + (uy.max(0.0)).powi(2) / (2.0 * G)
}

static ON_INCLINE: [Param; 3] = [
    param("speed", "Launch speed", "m/s", (2.0, 30.0), 0.5, 15.0),
    param(
        "elevation",
        "Launch angle above the slope",
        "°",
        (5.0, 80.0),
        1.0,
        30.0,
    ),
    param("slope", "Slope angle", "°", (5.0, 45.0), 1.0, 25.0),
];

fn projectile_on_incline(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let u = p.get(spec, "speed");
    let alpha = p.get(spec, "slope").to_radians();
    // Measured from the slope, and kept short of straight up so the ball always comes down on the slope.
    let beta = p
        .get(spec, "elevation")
        .min(85.0 - p.get(spec, "slope"))
        .to_radians();
    let theta = alpha + beta;
    let range = 2.0 * u * u * beta.sin() * theta.cos() / (G * alpha.cos().powi(2));
    let flight = 2.0 * u * beta.sin() / (G * alpha.cos());
    let radius = 0.25;
    // The ball starts resting just clear of the slope, so its centre flies and lands one radius above it:
    // the textbook's point projectile on a slope moved out by the radius.
    let (normal, _) = (Vec2::new(-alpha.sin(), alpha.cos()), ());
    let start = normal * (radius + LAUNCH_GAP);
    let run = 1.25 * range * alpha.cos() + 3.0;
    let apex = (u * theta.sin()).powi(2) / (2.0 * G);
    let top = apex.max(range * alpha.sin()) + 1.5;
    SceneFile::new(spec.title, [0.0, -G], spec.dt)
        .finish(FinishSpec::time(1.5 * flight + 1.0))
        .finish(FinishSpec::contact("ball", "slope"))
        .body(
            BodySpec::fixed(
                "slope",
                ShapeSpec::Polygon(vec![[0.0, 0.0], [run, 0.0], [run, run * alpha.tan()]]),
            )
            .friction(1.0, 1.0),
        )
        .body(
            ball("ball", 1.0, radius)
                .at(start.x, start.y)
                .velocity(u * theta.cos(), u * theta.sin())
                .restitution(0.0),
        )
        .focus("ball")
        .answer("Range along the slope (m)", range)
        .answer("Time of flight (s)", flight)
        .answer(
            "Greatest range at this speed (m)",
            u * u / (G * (1.0 + alpha.sin())),
        )
        .frame(
            [0.5 * range * alpha.cos(), 0.5 * top - 0.5],
            0.5 * range * alpha.cos() + 2.5,
            0.5 * top + 0.5,
        )
}

static CIRCULAR_MOTION: [Param; 3] = [
    param("radius", "Length of the string", "m", (0.3, 3.0), 0.1, 1.0),
    param("speed", "Speed", "m/s", (0.5, 10.0), 0.1, 3.0),
    param("mass", "Mass of the stone", "kg", (0.1, 5.0), 0.1, 0.5),
];

fn circular_motion(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let (r, v, m) = (
        p.get(spec, "radius"),
        p.get(spec, "speed"),
        p.get(spec, "mass"),
    );
    let period = std::f64::consts::TAU * r / v;
    // Seen from above, on a smooth table: gravity points into the page and does nothing in this plane.
    SceneFile::new(spec.title, [0.0, 0.0], spec.dt)
        .finish(FinishSpec::time(2.05 * period))
        .body(
            ball("stone", m, 0.08 * r.max(0.5))
                .at(r, 0.0)
                .velocity(0.0, v),
        )
        .constraint(ConstraintSpec::Rope {
            nodes: vec![RopeNodeSpec::Peg([0.0, 0.0]), RopeNodeSpec::attach("stone")],
            length: Some(r),
        })
        .focus("stone")
        .answer("Centripetal acceleration (m/s²)", v * v / r)
        .answer("Tension (N)", m * v * v / r)
        .answer("Period (s)", period)
        .view([0.0, 0.0], 1.5 * r)
}

// ---------------------------------------------------------------- Laws of motion

static INCLINE: [Param; 4] = [
    param("angle", "Incline angle", "°", (5.0, 60.0), 1.0, 30.0),
    param("mass", "Block mass", "kg", (0.5, 10.0), 0.5, 2.0),
    param(
        "static_friction",
        "Static friction μs",
        "",
        (0.0, 1.0),
        0.01,
        0.5,
    ),
    param(
        "kinetic_friction",
        "Kinetic friction μk",
        "",
        (0.0, 1.0),
        0.01,
        0.4,
    ),
];

fn incline_friction(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let theta = p.get(spec, "angle").to_radians();
    let (m, mu_s, mu_k) = (
        p.get(spec, "mass"),
        p.get(spec, "static_friction"),
        p.get(spec, "kinetic_friction"),
    );
    let length = 6.0;
    let center = on_incline(theta, length, 1.0, 0.2);
    let slides = theta.tan() > mu_s;
    let normal = m * G * theta.cos();
    scene(spec)
        .body(floor((mu_s, mu_k)))
        .body(incline(theta, length, (mu_s, mu_k)))
        .body(
            BodySpec::rigid("block", m, ShapeSpec::Rectangle([0.6, 0.4]))
                .at(center.x, center.y)
                .angle(-theta)
                .restitution(0.0),
        )
        .focus("block")
        .finish(FinishSpec::contact("block", "ground"))
        .finish(FinishSpec::rest(0.5))
        .answer("tan θ", theta.tan())
        .answer("Slides? (1 = yes)", if slides { 1.0 } else { 0.0 })
        .answer(
            "Acceleration (m/s²)",
            if slides {
                exact::incline_sliding_acceleration(theta, mu_k, G)
            } else {
                0.0
            },
        )
        .answer("Normal force (N)", normal)
        .answer(
            "Friction (N)",
            if slides {
                mu_k * normal
            } else {
                m * G * theta.sin()
            },
        )
        .view(
            [0.5 * length, 0.5 * length * theta.tan()],
            0.5 * length * theta.tan().max(0.6) + 1.5,
        )
}

static PUSHED: [Param; 4] = [
    param("push", "Push", "N", (0.0, 60.0), 0.5, 8.0),
    param("mass", "Block mass", "kg", (0.5, 10.0), 0.5, 2.0),
    param(
        "static_friction",
        "Static friction μs",
        "",
        (0.0, 1.0),
        0.01,
        0.5,
    ),
    param(
        "kinetic_friction",
        "Kinetic friction μk",
        "",
        (0.0, 1.0),
        0.01,
        0.3,
    ),
];

fn pushed_block(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let (push, m, mu_s, mu_k) = (
        p.get(spec, "push"),
        p.get(spec, "mass"),
        p.get(spec, "static_friction"),
        p.get(spec, "kinetic_friction"),
    );
    let threshold = mu_s * m * G;
    let moves = push > threshold;
    scene(spec)
        .body(floor((mu_s, mu_k)))
        .body(BodySpec::rigid("block", m, ShapeSpec::Rectangle([0.8, 0.5])).at(0.0, 0.25))
        .force(ForceSpec::Applied {
            body: "block".into(),
            force: [push, 0.0],
            point: [-0.4, 0.0],
            from: None,
            until: None,
        })
        .focus("block")
        .finish(FinishSpec::rest(0.5))
        .answer("Push needed to move it (N)", threshold)
        .answer("Moves? (1 = yes)", if moves { 1.0 } else { 0.0 })
        .answer(
            "Acceleration (m/s²)",
            if moves {
                (push - mu_k * m * G) / m
            } else {
                0.0
            },
        )
        .answer("Friction (N)", if moves { mu_k * m * G } else { push })
        .view([3.0, 1.0], 2.5)
}

static ATWOOD: [Param; 3] = [
    param("m1", "Left mass", "kg", (0.1, 10.0), 0.1, 2.0),
    param("m2", "Right mass", "kg", (0.1, 10.0), 0.1, 3.0),
    param(
        "height",
        "Pulley height above the masses",
        "m",
        (1.0, 8.0),
        0.1,
        5.0,
    ),
];

fn atwood(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let (m1, m2, height) = (p.get(spec, "m1"), p.get(spec, "m2"), p.get(spec, "height"));
    let radius = 0.4;
    // The masses hang `height` below the pulley and have 2.5 m to fall before the floor.
    let floor = -(height + 2.5);
    scene(spec)
        .body(
            BodySpec::fixed(
                "floor",
                ShapeSpec::Halfplane {
                    normal: [0.0, 1.0],
                    offset: floor,
                },
            )
            .friction(0.5, 0.4),
        )
        .body(ball("m1", m1, 0.2).at(-radius, -height).restitution(0.0))
        .body(ball("m2", m2, 0.2).at(radius, -height).restitution(0.0))
        .constraint(ConstraintSpec::Rope {
            nodes: vec![
                RopeNodeSpec::attach("m1"),
                RopeNodeSpec::Pulley {
                    center: [0.0, 0.0],
                    radius,
                    wrap: WrapSpec::Clockwise,
                },
                RopeNodeSpec::attach("m2"),
            ],
            length: None,
        })
        .focus(if m2 >= m1 { "m2" } else { "m1" })
        .finish(FinishSpec::contact("m1", "floor"))
        .finish(FinishSpec::contact("m2", "floor"))
        .finish(FinishSpec::rest(0.5))
        .answer(
            "Acceleration (m/s²)",
            exact::atwood_acceleration(m1, m2, G).abs(),
        )
        .answer("Tension (N)", exact::atwood_tension(m1, m2, G))
        .view([0.0, 0.5 * (0.9 + floor - 0.5)], 0.5 * (0.9 - floor + 0.5))
}

static TABLE: [Param; 3] = [
    param("m1", "Block on table", "kg", (0.1, 10.0), 0.1, 4.0),
    param("m2", "Hanging mass", "kg", (0.1, 10.0), 0.1, 1.0),
    param("friction", "Table friction μ", "", (0.0, 1.0), 0.01, 0.0),
];

fn table_and_hanging_mass(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let (m1, m2, mu) = (
        p.get(spec, "m1"),
        p.get(spec, "m2"),
        p.get(spec, "friction"),
    );
    let (radius, edge) = (0.15, 4.0);
    let moves = m2 > mu * m1;
    let a = if moves {
        (m2 - mu * m1) / (m1 + m2) * G
    } else {
        0.0
    };
    scene(spec)
        .body(
            BodySpec::fixed(
                "table",
                ShapeSpec::Polygon(vec![[-3.0, -0.4], [edge, -0.4], [edge, 0.0], [-3.0, 0.0]]),
            )
            .friction(mu, mu),
        )
        .body(
            BodySpec::fixed(
                "floor",
                ShapeSpec::Halfplane {
                    normal: [0.0, 1.0],
                    offset: -3.5,
                },
            )
            .friction(0.5, 0.4),
        )
        .body(BodySpec::rigid("block", m1, ShapeSpec::Rectangle([0.8, 0.5])).at(0.0, 0.25))
        .body(
            ball("hanging mass", m2, 0.2)
                .at(edge + radius, -2.0)
                .restitution(0.0),
        )
        .constraint(ConstraintSpec::Rope {
            nodes: vec![
                RopeNodeSpec::Attach {
                    body: "block".into(),
                    local: [0.4, 0.0],
                },
                RopeNodeSpec::Pulley {
                    center: [edge, 0.25 - radius],
                    radius,
                    wrap: WrapSpec::Clockwise,
                },
                RopeNodeSpec::attach("hanging mass"),
            ],
            length: None,
        })
        .focus("block")
        .finish(FinishSpec::contact("hanging mass", "floor"))
        .finish(FinishSpec::rest(0.5))
        .answer("Moves? (1 = yes)", if moves { 1.0 } else { 0.0 })
        .answer("Acceleration (m/s²)", a)
        .answer(
            "Tension (N)",
            if moves { m1 * (a + mu * G) } else { m2 * G },
        )
        .answer("Normal force on block (N)", m1 * G)
        .view([1.0, -0.8], 3.0)
}

static CONNECTED: [Param; 4] = [
    param("m1", "Block behind", "kg", (0.5, 10.0), 0.5, 2.0),
    param("m2", "Block in front", "kg", (0.5, 10.0), 0.5, 3.0),
    param("force", "Pull", "N", (0.0, 60.0), 0.5, 10.0),
    param("friction", "Floor friction μ", "", (0.0, 0.8), 0.01, 0.0),
];

fn connected_blocks(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let (m1, m2, pull, mu) = (
        p.get(spec, "m1"),
        p.get(spec, "m2"),
        p.get(spec, "force"),
        p.get(spec, "friction"),
    );
    let total = m1 + m2;
    let needed = mu * total * G;
    let moves = pull > needed;
    let block = |id: &str, mass: f64, x: f64| {
        BodySpec::rigid(id, mass, ShapeSpec::Rectangle([0.8, 0.5]))
            .at(x, 0.25)
            .restitution(0.0)
    };
    let scene = scene(spec)
        .body(floor((mu, mu)))
        .body(block("block behind", m1, 0.0))
        .body(block("block in front", m2, 1.6))
        .constraint(ConstraintSpec::Rope {
            nodes: vec![
                RopeNodeSpec::Attach {
                    body: "block behind".into(),
                    local: [0.4, 0.0],
                },
                RopeNodeSpec::Attach {
                    body: "block in front".into(),
                    local: [-0.4, 0.0],
                },
            ],
            length: None,
        })
        .force(ForceSpec::Applied {
            body: "block in front".into(),
            force: [pull, 0.0],
            point: [0.4, 0.0],
            from: None,
            until: None,
        })
        .focus("block behind")
        .finish(FinishSpec::rest(0.5))
        .answer("Moves? (1 = yes)", if moves { 1.0 } else { 0.0 })
        .answer(
            "Acceleration (m/s²)",
            if moves { pull / total - mu * G } else { 0.0 },
        );
    // Friction takes the same share from each block as the pull gives, so the string's tension does not depend on
    // it. At rest the tension is not fixed by the laws of motion alone, and no answer is given.
    let scene = if moves {
        scene.answer("Tension (N)", m1 * pull / total)
    } else {
        scene.answer("Pull needed to move them (N)", needed)
    };
    scene.frame([3.0, 1.0], 5.5, 2.5)
}

static LIFT: [Param; 2] = [
    param("mass", "Mass of the person", "kg", (20.0, 120.0), 1.0, 60.0),
    param(
        "acceleration",
        "Acceleration of the lift, up is +",
        "m/s²",
        (-9.8, 5.0),
        0.1,
        2.0,
    ),
];

fn lift(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let (m, a) = (p.get(spec, "mass"), p.get(spec, "acceleration"));
    let cabin = 400.0;
    let apparent = m * (G + a);
    // The lift's floor is held upright on a vertical track and hauled by a cable pulling with exactly the force
    // that gives floor and passenger the chosen acceleration. What the floor then pushes on the person with is
    // what a weighing scale under their feet would read.
    scene(spec)
        .body(
            BodySpec::rigid("lift floor", cabin, ShapeSpec::Rectangle([2.0, 0.2]))
                .friction(0.8, 0.6)
                .restitution(0.0),
        )
        .body(
            BodySpec::rigid("person", m, ShapeSpec::Rectangle([0.5, 1.0]))
                .at(0.0, 0.6)
                .restitution(0.0),
        )
        .constraint(ConstraintSpec::Slot {
            body: "lift floor".into(),
            local: [0.0, 0.0],
            origin: [0.0, 0.0],
            direction: [0.0, 1.0],
        })
        .constraint(ConstraintSpec::FixedAngle {
            body: "lift floor".into(),
            angle: 0.0,
        })
        .force(ForceSpec::Applied {
            body: "lift floor".into(),
            force: [0.0, (cabin + m) * (G + a)],
            point: [0.0, 0.0],
            from: None,
            until: None,
        })
        .focus("person")
        .answer("True weight (N)", m * G)
        .answer("Apparent weight (N)", apparent)
        .answer("Scale reading (kg)", apparent / G)
        .view([0.0, 1.0], 3.0)
}

static RECOIL: [Param; 3] = [
    param("gun", "Mass of the gun", "kg", (1.0, 20.0), 0.5, 5.0),
    param("shell", "Mass of the shell", "kg", (0.05, 2.0), 0.05, 0.5),
    param("speed", "Speed of the shell", "m/s", (5.0, 60.0), 1.0, 20.0),
];

/// How long the charge pushes, seconds. Its length does not matter to the answer, only the impulse does.
const FIRING_TIME: f64 = 0.02;

fn recoil(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let (gun, shell, v) = (
        p.get(spec, "gun"),
        p.get(spec, "shell"),
        p.get(spec, "speed"),
    );
    // The charge pushes shell and gun apart with equal and opposite forces for a moment: Newton's third law.
    let push = shell * v / FIRING_TIME;
    let fire = |body: &str, force: f64| ForceSpec::Applied {
        body: body.into(),
        force: [force, 0.0],
        point: [0.0, 0.0],
        from: None,
        until: Some(FIRING_TIME),
    };
    scene(spec)
        .body(floor((0.0, 0.0)))
        .body(
            BodySpec::rigid("gun", gun, ShapeSpec::Rectangle([1.2, 0.4]))
                .at(0.0, 0.2)
                .restitution(0.0),
        )
        .body(ball("shell", shell, 0.1).at(0.75, 0.1).restitution(0.0))
        .force(fire("shell", push))
        .force(fire("gun", -push))
        .focus("gun")
        .answer("Recoil speed of the gun (m/s)", shell * v / gun)
        .answer("Momentum of the shell (kg·m/s)", shell * v)
        .answer("Total momentum (kg·m/s)", 0.0)
        .answer(
            "Share of the energy the shell carries (%)",
            100.0 * gun / (gun + shell),
        )
        .frame([2.0, 0.6], 5.0, 2.2)
}

// ---------------------------------------------------------------- Work, energy and power

static COLLISION: [Param; 5] = [
    param("m1", "Mass 1", "kg", (0.1, 10.0), 0.1, 1.0),
    param("u1", "Speed 1", "m/s", (-10.0, 10.0), 0.1, 3.0),
    param("m2", "Mass 2", "kg", (0.1, 10.0), 0.1, 2.0),
    param("u2", "Speed 2", "m/s", (-10.0, 10.0), 0.1, 0.0),
    param(
        "restitution",
        "Coefficient of restitution e",
        "",
        (0.0, 1.0),
        0.01,
        1.0,
    ),
];

fn collision_1d(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let (m1, u1, m2, u2, e) = (
        p.get(spec, "m1"),
        p.get(spec, "u1"),
        p.get(spec, "m2"),
        p.get(spec, "u2"),
        p.get(spec, "restitution"),
    );
    let (v1, v2) = exact::collision_1d(m1, u1, m2, u2, e);
    let lost = 0.5 * m1 * u1 * u1 + 0.5 * m2 * u2 * u2 - 0.5 * m1 * v1 * v1 - 0.5 * m2 * v2 * v2;
    scene(spec)
        .body(floor((0.0, 0.0)))
        .body(
            ball("mass 1", m1, 0.3)
                .at(-3.0, 0.3)
                .velocity(u1, 0.0)
                .restitution(e),
        )
        .body(
            ball("mass 2", m2, 0.3)
                .at(0.0, 0.3)
                .velocity(u2, 0.0)
                .restitution(e),
        )
        .focus("mass 1")
        .finish(FinishSpec::contact("mass 1", "mass 2").hold(1.0))
        .answer("Final speed 1 (m/s)", v1)
        .answer("Final speed 2 (m/s)", v2)
        .answer("Kinetic energy lost (J)", lost)
        .view([0.0, 0.5], 3.0)
}

static VERTICAL_CIRCLE: [Param; 3] = [
    param("length", "String length", "m", (0.3, 3.0), 0.1, 1.0),
    param("speed", "Speed at the bottom", "m/s", (0.5, 15.0), 0.1, 7.0),
    param("mass", "Bob mass", "kg", (0.1, 5.0), 0.1, 0.5),
];

fn vertical_circle(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let (length, u, m) = (
        p.get(spec, "length"),
        p.get(spec, "speed"),
        p.get(spec, "mass"),
    );
    scene(spec)
        .body(
            ball("bob", m, 0.08 * length)
                .at(0.0, -length)
                .velocity(u, 0.0),
        )
        .constraint(ConstraintSpec::Rope {
            nodes: vec![RopeNodeSpec::Peg([0.0, 0.0]), RopeNodeSpec::attach("bob")],
            length: Some(length),
        })
        .focus("bob")
        .answer(
            "Speed needed to complete the circle (m/s)",
            exact::vertical_circle_min_speed(length, G),
        )
        .answer("Tension at the bottom (N)", m * (u * u / length + G))
        .answer(
            "Tension at the top if it gets there (N)",
            (m * (u * u / length - 5.0 * G)).max(0.0),
        )
        .view([0.0, 0.0], 1.4 * length)
}

static SPRING_STOP: [Param; 3] = [
    param("mass", "Mass of the block", "kg", (0.5, 5.0), 0.1, 1.0),
    param("speed", "Speed of the block", "m/s", (0.5, 6.0), 0.1, 3.0),
    param(
        "stiffness",
        "Spring constant k",
        "N/m",
        (20.0, 500.0),
        5.0,
        100.0,
    ),
];

fn spring_compression(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let (m, u, k) = (
        p.get(spec, "mass"),
        p.get(spec, "speed"),
        p.get(spec, "stiffness"),
    );
    let compression = u * (m / k).sqrt();
    let (natural, gap) = (2.0, 1.5);
    // Half an oscillation on the spring, between meeting it and leaving it.
    let squeeze = std::f64::consts::PI * (m / k).sqrt();
    SceneFile::new(spec.title, [0.0, -G], spec.dt)
        .finish(FinishSpec::time(gap / u + squeeze + 0.6))
        .body(floor((0.0, 0.0)))
        .body(BodySpec::fixed(
            "wall",
            ShapeSpec::Polygon(vec![[-0.3, 0.0], [0.0, 0.0], [0.0, 1.0], [-0.3, 1.0]]),
        ))
        .body(
            BodySpec::rigid("block", m, ShapeSpec::Rectangle([0.6, 0.5]))
                .at(natural + 0.3 + gap, 0.25)
                .velocity(-u, 0.0)
                .restitution(0.0),
        )
        .constraint(ConstraintSpec::Slot {
            body: "block".into(),
            local: [0.0, 0.0],
            origin: [0.0, 0.25],
            direction: [1.0, 0.0],
        })
        // The spring's free end waits at its natural length and pushes only while the block squeezes it.
        .force(ForceSpec::Spring {
            a: AnchorSpec::Fixed([0.0, 0.25]),
            b: AnchorSpec::body_at("block", [-0.3, 0.0]),
            stiffness: k,
            rest_length: Some(natural),
            pushes_only: true,
        })
        .focus("block")
        .answer("Greatest compression (m)", compression)
        .answer("Greatest force of the spring (N)", k * compression)
        .answer("Speed it comes back with (m/s)", u)
        .frame([2.2, 0.6], 2.8, 2.2)
}

static DROP_SPRING: [Param; 3] = [
    param("mass", "Mass of the ball", "kg", (0.2, 5.0), 0.1, 1.0),
    param(
        "height",
        "Drop height above the spring",
        "m",
        (0.2, 3.0),
        0.1,
        1.0,
    ),
    param(
        "stiffness",
        "Spring constant k",
        "N/m",
        (100.0, 2000.0),
        50.0,
        500.0,
    ),
];

fn ball_on_spring(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let (m, h, k) = (
        p.get(spec, "mass"),
        p.get(spec, "height"),
        p.get(spec, "stiffness"),
    );
    // ½ k x² = m g (h + x): the ball falls h to the spring and then x more while squeezing it.
    let weight = m * G;
    let rest = weight / k;
    let compression = (weight + (weight * weight + 2.0 * k * weight * h).sqrt()) / k;
    let (natural, radius) = (1.2, 0.15);
    // On the spring the ball is an oscillator about where it would rest, entered at speed √(2gh). It is back at
    // the spring's free end after this long.
    let (omega, arrival) = ((k / m).sqrt(), (2.0 * G * h).sqrt());
    let squeeze = 2.0 * (arrival / omega).atan2(-rest) / omega;
    SceneFile::new(spec.title, [0.0, -G], spec.dt)
        // It ends a moment after the ball has left the spring, while it is still on its way back up.
        .finish(FinishSpec::time(
            (2.0 * h / G).sqrt() + squeeze + (0.6 * arrival / G).min(0.3),
        ))
        .body(floor((0.0, 0.0)))
        .body(
            ball("ball", m, radius)
                .at(0.0, natural + h + radius)
                .restitution(0.0),
        )
        .constraint(ConstraintSpec::Slot {
            body: "ball".into(),
            local: [0.0, 0.0],
            origin: [0.0, 0.0],
            direction: [0.0, 1.0],
        })
        .force(ForceSpec::Spring {
            a: AnchorSpec::Fixed([0.0, 0.0]),
            b: AnchorSpec::body_at("ball", [0.0, -radius]),
            stiffness: k,
            rest_length: Some(natural),
            pushes_only: true,
        })
        .focus("ball")
        .answer("Speed on reaching the spring (m/s)", arrival)
        .answer("Greatest compression (m)", compression)
        .answer("Compression it would rest at (m)", rest)
        .view([0.0, 0.5 * (natural + h) + 0.3], 0.5 * (natural + h) + 0.9)
}

static OBLIQUE: [Param; 2] = [
    param(
        "speed",
        "Speed of the incoming disc",
        "m/s",
        (1.0, 10.0),
        0.1,
        4.0,
    ),
    param(
        "offset",
        "Off-centre by, as a fraction of a diameter",
        "",
        (0.0, 0.95),
        0.01,
        0.5,
    ),
];

fn oblique_collision(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let (u, offset) = (p.get(spec, "speed"), p.get(spec, "offset"));
    let radius = 0.3;
    // At the moment they touch the line of centres makes this angle with the incoming path, and a smooth disc
    // can only be pushed along that line.
    let struck = offset.asin();
    let disc = |id: &str, x: f64, y: f64, vx: f64| {
        BodySpec::rigid(id, 1.0, ShapeSpec::Circle(radius))
            .at(x, y)
            .velocity(vx, 0.0)
    };
    // Seen from above, on a smooth table.
    SceneFile::new(spec.title, [0.0, 0.0], spec.dt)
        .finish(FinishSpec::time(6.0))
        .finish(FinishSpec::contact("incoming disc", "struck disc").hold(1.2))
        .body(disc("incoming disc", -3.0, offset * 2.0 * radius, u))
        .body(disc("struck disc", 0.0, 0.0, 0.0))
        .focus("incoming disc")
        .answer("Speed of the struck disc (m/s)", u * struck.cos())
        .answer("Speed of the incoming disc (m/s)", u * struck.sin())
        .answer(
            "Angle of the struck disc from the incoming path (°)",
            struck.to_degrees(),
        )
        .answer("Angle between the two afterwards (°)", 90.0)
        .frame([0.0, 0.0], 5.0, 3.5)
}

// ---------------------------------------------------------------- Rotation

static ROLLING: [Param; 3] = [
    param("angle", "Incline angle", "°", (5.0, 50.0), 1.0, 25.0),
    param(
        "inertia_ratio",
        "I / mR² (0.5 disc, 1 ring)",
        "",
        (0.1, 1.0),
        0.05,
        0.5,
    ),
    param(
        "static_friction",
        "Static friction μs",
        "",
        (0.0, 1.0),
        0.01,
        0.3,
    ),
];

fn rolling(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let theta = p.get(spec, "angle").to_radians();
    let (k, mu_s) = (p.get(spec, "inertia_ratio"), p.get(spec, "static_friction"));
    let (length, radius) = (8.0, 0.4);
    let center = on_incline(theta, length, 1.0, radius);
    let needed = exact::rolling_min_friction(theta, k);
    let rolls = mu_s >= needed;
    scene(spec)
        .body(floor((mu_s, 0.9 * mu_s)))
        .body(incline(theta, length, (mu_s, 0.9 * mu_s)))
        .body(
            BodySpec::rigid("disc", 1.0, ShapeSpec::Circle(radius))
                .at(center.x, center.y)
                .inertia(k * radius * radius)
                .restitution(0.0),
        )
        .focus("disc")
        .finish(FinishSpec::contact("disc", "ground"))
        .finish(FinishSpec::rest(0.5))
        .answer("Friction needed to roll without slipping", needed)
        .answer(
            "Rolls without slipping? (1 = yes)",
            if rolls { 1.0 } else { 0.0 },
        )
        .answer(
            "Acceleration (m/s²)",
            if rolls {
                exact::rolling_acceleration(theta, k, G)
            } else {
                exact::incline_sliding_acceleration(theta, 0.9 * mu_s, G)
            },
        )
        .view(
            [0.5 * length, 0.5 * length * theta.tan()],
            0.5 * length * theta.tan().max(0.5) + 1.5,
        )
}

static ATWOOD_MASSIVE: [Param; 3] = [
    param("m1", "Left mass", "kg", (0.1, 10.0), 0.1, 2.0),
    param("m2", "Right mass", "kg", (0.1, 10.0), 0.1, 3.0),
    param(
        "pulley_mass",
        "Pulley mass (uniform disc)",
        "kg",
        (0.0, 20.0),
        0.5,
        4.0,
    ),
];

fn atwood_massive(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let (m1, m2, pulley_mass) = (
        p.get(spec, "m1"),
        p.get(spec, "m2"),
        p.get(spec, "pulley_mass").max(1e-6),
    );
    let radius = 0.4;
    let a = exact::atwood_massive_acceleration(m1, m2, pulley_mass, G);
    scene(spec)
        .body(
            BodySpec::fixed(
                "floor",
                ShapeSpec::Halfplane {
                    normal: [0.0, 1.0],
                    offset: -7.5,
                },
            )
            .friction(0.5, 0.4),
        )
        .body(BodySpec::rigid(
            "pulley",
            pulley_mass,
            ShapeSpec::Circle(radius),
        ))
        .body(ball("m1", m1, 0.2).at(-radius, -5.0).restitution(0.0))
        .body(ball("m2", m2, 0.2).at(radius, -5.0).restitution(0.0))
        .constraint(ConstraintSpec::Pin {
            a: AnchorSpec::body("pulley"),
            b: AnchorSpec::Fixed([0.0, 0.0]),
        })
        .constraint(ConstraintSpec::Rope {
            nodes: vec![
                RopeNodeSpec::attach("m1"),
                RopeNodeSpec::Wheel {
                    body: "pulley".into(),
                    radius,
                    wrap: WrapSpec::Clockwise,
                },
                RopeNodeSpec::attach("m2"),
            ],
            length: None,
        })
        .focus(if m2 >= m1 { "m2" } else { "m1" })
        .finish(FinishSpec::contact("m1", "floor"))
        .finish(FinishSpec::contact("m2", "floor"))
        .finish(FinishSpec::rest(0.5))
        .answer("Acceleration (m/s²)", a.abs())
        .answer("Tension on the left (N)", m1 * (G + a))
        .answer("Tension on the right (N)", m2 * (G - a))
        .answer(
            "Angular acceleration of the pulley (rad/s²)",
            a.abs() / radius,
        )
        .view([0.0, -3.8], 4.3)
}

static FLYWHEEL: [Param; 3] = [
    param(
        "wheel_mass",
        "Wheel mass (uniform disc)",
        "kg",
        (0.5, 50.0),
        0.5,
        10.0,
    ),
    param("radius", "Wheel radius", "m", (0.1, 1.0), 0.05, 0.5),
    param("mass", "Hanging mass", "kg", (0.1, 10.0), 0.1, 2.0),
];

fn flywheel(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let (wheel_mass, radius, m) = (
        p.get(spec, "wheel_mass"),
        p.get(spec, "radius"),
        p.get(spec, "mass"),
    );
    let a = m * G / (m + 0.5 * wheel_mass);
    scene(spec)
        .body(
            BodySpec::fixed(
                "floor",
                ShapeSpec::Halfplane {
                    normal: [0.0, 1.0],
                    offset: -9.0,
                },
            )
            .friction(0.5, 0.4),
        )
        .body(BodySpec::rigid(
            "wheel",
            wheel_mass,
            ShapeSpec::Circle(radius),
        ))
        .body(ball("mass", m, 0.15).at(radius, -2.5).restitution(0.0))
        .constraint(ConstraintSpec::Pin {
            a: AnchorSpec::body("wheel"),
            b: AnchorSpec::Fixed([0.0, 0.0]),
        })
        .constraint(ConstraintSpec::Rope {
            nodes: vec![
                RopeNodeSpec::Wheel {
                    body: "wheel".into(),
                    radius,
                    wrap: WrapSpec::Clockwise,
                },
                RopeNodeSpec::attach("mass"),
            ],
            length: None,
        })
        .focus("mass")
        .finish(FinishSpec::contact("mass", "floor"))
        .answer("Acceleration of the mass (m/s²)", a)
        .answer("Tension (N)", m * (G - a))
        .answer("Angular acceleration of the wheel (rad/s²)", a / radius)
        .view([0.0, -4.0], 5.2)
}

static SEESAW: [Param; 4] = [
    param("m1", "Left mass", "kg", (0.5, 10.0), 0.5, 3.0),
    param("d1", "Left distance", "m", (0.2, 2.0), 0.1, 1.0),
    param("m2", "Right mass", "kg", (0.5, 10.0), 0.5, 2.0),
    param("d2", "Right distance", "m", (0.2, 2.0), 0.1, 1.5),
];

fn seesaw(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let (m1, d1, m2, d2) = (
        p.get(spec, "m1"),
        p.get(spec, "d1"),
        p.get(spec, "m2"),
        p.get(spec, "d2"),
    );
    let (r1, r2) = (0.15 + 0.03 * m1, 0.15 + 0.03 * m2);
    let plank_inertia = 2.0 * (4.4f64.powi(2) + 0.1f64.powi(2)) / 12.0;
    let inertia = plank_inertia + m1 * (0.05 + r1).powi(2) + m2 * (0.05 + r2).powi(2);
    scene(spec)
        .body(BodySpec::rigid("plank", 2.0, ShapeSpec::Rectangle([4.4, 0.1])).at(0.0, 1.0))
        .body(ball("m1", m1, r1).at(-d1, 1.05 + r1))
        .body(ball("m2", m2, r2).at(d2, 1.05 + r2))
        .constraint(ConstraintSpec::Pin {
            a: AnchorSpec::body("plank"),
            b: AnchorSpec::Fixed([0.0, 1.0]),
        })
        .constraint(ConstraintSpec::Pin {
            a: AnchorSpec::body("m1"),
            b: AnchorSpec::body_at("plank", [-d1, 0.05 + r1]),
        })
        .constraint(ConstraintSpec::Pin {
            a: AnchorSpec::body("m2"),
            b: AnchorSpec::body_at("plank", [d2, 0.05 + r2]),
        })
        .focus("plank")
        .finish(FinishSpec::rest(0.5))
        .answer("Left torque m1 d1 g (N·m)", m1 * d1 * G)
        .answer("Right torque m2 d2 g (N·m)", m2 * d2 * G)
        .answer(
            "Initial angular acceleration (rad/s²)",
            exact::seesaw_angular_acceleration(m1, d1, m2, d2, inertia, G),
        )
        .view([0.0, 1.0], 2.5)
}

static TORQUE_ROD: [Param; 3] = [
    param("torque", "Torque", "N·m", (0.5, 20.0), 0.5, 4.0),
    param("mass", "Mass of the rod", "kg", (0.5, 10.0), 0.5, 2.0),
    param("length", "Length of the rod", "m", (0.5, 3.0), 0.1, 2.0),
];

/// How long the rod is turned for, seconds.
const TURNING_TIME: f64 = 2.0;

fn torque_on_rod(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let (torque, m, length) = (
        p.get(spec, "torque"),
        p.get(spec, "mass"),
        p.get(spec, "length"),
    );
    // A thin uniform rod about one end: M L²/12 about its middle, plus M (L/2)² for moving the axis to the end.
    let about_pivot = m * length * length / 3.0;
    let alpha = torque / about_pivot;
    // Seen from above, on a smooth table, pivoted at one end.
    SceneFile::new(spec.title, [0.0, 0.0], spec.dt)
        .finish(FinishSpec::time(TURNING_TIME))
        .body(
            BodySpec::rigid("rod", m, ShapeSpec::Rectangle([length, 0.08]))
                .at(0.5 * length, 0.0)
                .inertia(m * length * length / 12.0),
        )
        .constraint(ConstraintSpec::Pin {
            a: AnchorSpec::body_at("rod", [-0.5 * length, 0.0]),
            b: AnchorSpec::Fixed([0.0, 0.0]),
        })
        .force(ForceSpec::Torque {
            body: "rod".into(),
            torque,
        })
        .focus("rod")
        .answer("Moment of inertia about the pivot (kg·m²)", about_pivot)
        .answer("Angular acceleration (rad/s²)", alpha)
        .answer("Angular speed after 2 s (rad/s)", alpha * TURNING_TIME)
        .answer(
            "Turns made in 2 s",
            0.5 * alpha * TURNING_TIME * TURNING_TIME / std::f64::consts::TAU,
        )
        .view([0.0, 0.0], 1.3 * length)
}

// ---------------------------------------------------------------- Gravitation

const BIG_G: f64 = 6.674e-11;
const EARTH_MASS: f64 = 5.972e24;
const EARTH_RADIUS: f64 = 6.371e6;
const EARTH_MU: f64 = BIG_G * EARTH_MASS;
const KM: f64 = 1000.0;

/// The largest of 1, 2, 5 times a power of ten that does not exceed `value`: a time step people can read.
fn round_step(value: f64) -> f64 {
    let magnitude = 10f64.powf(value.log10().floor());
    [5.0, 2.0, 1.0]
        .into_iter()
        .map(|m| m * magnitude)
        .find(|&step| step <= value)
        .unwrap_or(magnitude)
}

/// A scene in space around the Earth: no uniform gravity, Newton's law from the Earth's mass at the origin,
/// and the planet drawn as a fixed disc. `duration` is how long the problem lasts; the scene is stepped about
/// six thousand times over it and played back in about fifteen seconds.
fn around_earth(spec: &ScenarioSpec, duration: f64, step: (f64, f64)) -> SceneFile {
    let dt = round_step(duration / 6000.0).clamp(step.0, step.1);
    SceneFile::new(spec.title, [0.0, 0.0], dt)
        .playback(duration / 15.0)
        .finish(FinishSpec::time(duration))
        .body(BodySpec::fixed("Earth", ShapeSpec::Circle(EARTH_RADIUS)).restitution(0.0))
        .force(ForceSpec::Gravitation {
            constant: BIG_G,
            fixed: vec![SourceSpec {
                at: [0.0, 0.0],
                mass: EARTH_MASS,
            }],
        })
}

fn satellite(id: &str) -> BodySpec {
    BodySpec::particle(id, 1000.0).restitution(0.0)
}

static CIRCULAR_ORBIT: [Param; 1] = [param(
    "altitude",
    "Altitude",
    "km",
    (200.0, 36000.0),
    100.0,
    400.0,
)];

fn circular_orbit(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let r = EARTH_RADIUS + p.get(spec, "altitude") * KM;
    let speed = exact::circular_orbit_speed(EARTH_MU, r);
    let period = exact::orbit_period(EARTH_MU, r);
    // A little over two turns, so the period can be read off between two passes of the same point.
    around_earth(spec, 2.05 * period, (0.5, 60.0))
        .body(satellite("satellite").at(r, 0.0).velocity(0.0, speed))
        .focus("satellite")
        .answer("Orbital speed (km/s)", speed / KM)
        .answer("Period (min)", period / 60.0)
        .answer("Acceleration (m/s²)", EARTH_MU / (r * r))
        .view([0.0, 0.0], 1.2 * r)
}

static ELLIPTICAL_ORBIT: [Param; 2] = [
    param(
        "perigee",
        "Altitude at the nearest point",
        "km",
        (200.0, 2000.0),
        50.0,
        400.0,
    ),
    param(
        "boost",
        "Speed there ÷ circular speed",
        "",
        (1.0, 1.35),
        0.01,
        1.2,
    ),
];

fn elliptical_orbit(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let r_p = EARTH_RADIUS + p.get(spec, "perigee") * KM;
    let v_p = p.get(spec, "boost") * exact::circular_orbit_speed(EARTH_MU, r_p);
    let (a, e) = exact::orbit_from_perigee(EARTH_MU, r_p, v_p);
    let r_a = a * (1.0 + e);
    let period = exact::orbit_period(EARTH_MU, a);
    let minor = a * (1.0 - e * e).sqrt();
    around_earth(spec, 2.05 * period, (0.5, 60.0))
        .body(satellite("satellite").at(r_p, 0.0).velocity(0.0, v_p))
        .focus("satellite")
        .answer("Farthest altitude (km)", (r_a - EARTH_RADIUS) / KM)
        .answer("Period (min)", period / 60.0)
        .answer("Speed at the nearest point (km/s)", v_p / KM)
        // Kepler's second law: r v is the same at both ends of the ellipse.
        .answer("Speed at the farthest point (km/s)", v_p * r_p / r_a / KM)
        .answer("Eccentricity", e)
        .frame(
            [0.5 * (r_p - r_a), 0.0],
            1.15 * a,
            1.15 * minor.max(EARTH_RADIUS),
        )
}

static ESCAPE: [Param; 1] = [param(
    "speed",
    "Launch speed",
    "km/s",
    (1.0, 15.0),
    0.1,
    8.0,
)];

/// The longest a launch is followed, seconds. A body thrown just below the escape speed takes days to return.
const LONGEST_FLIGHT: f64 = 6.0 * 3600.0;

fn escape_velocity(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let speed = p.get(spec, "speed") * KM;
    let start = EARTH_RADIUS + 10.0;
    let escape = exact::escape_speed(EARTH_MU, start);
    let escapes = speed >= escape;
    let (height, duration) = if escapes {
        (f64::INFINITY, 3.0 * 3600.0)
    } else {
        let height = exact::vertical_launch_height(EARTH_MU, start, speed);
        let rise = exact::vertical_launch_rise_time(EARTH_MU, start, start + height);
        (height, (2.2 * rise).min(LONGEST_FLIGHT))
    };
    let shown = height.min(12.0 * EARTH_RADIUS);
    let (top, bottom) = (EARTH_RADIUS + 1.1 * shown, -1.1 * EARTH_RADIUS);
    let scene = around_earth(spec, duration, (0.5, 2.0))
        .body(satellite("rocket").at(0.0, start).velocity(0.0, speed))
        .focus("rocket")
        .finish(FinishSpec::contact("rocket", "Earth"))
        .answer("Escape speed (km/s)", escape / KM)
        .answer("Escapes? (1 = yes)", if escapes { 1.0 } else { 0.0 });
    let scene = if escapes {
        scene.answer(
            "Speed left when far away (km/s)",
            (speed * speed - escape * escape).sqrt() / KM,
        )
    } else {
        scene.answer("Maximum height (km)", height / KM)
    };
    scene.view([0.0, 0.5 * (top + bottom)], 0.5 * (top - bottom))
}

// ---------------------------------------------------------------- Oscillations

static SPRING_MASS: [Param; 3] = [
    param("mass", "Mass", "kg", (0.1, 5.0), 0.1, 1.0),
    param(
        "stiffness",
        "Spring constant k",
        "N/m",
        (1.0, 100.0),
        1.0,
        20.0,
    ),
    param("amplitude", "Amplitude", "m", (0.1, 1.5), 0.05, 0.6),
];

fn spring_mass(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let (m, k, amplitude) = (
        p.get(spec, "mass"),
        p.get(spec, "stiffness"),
        p.get(spec, "amplitude"),
    );
    let rest = 2.0;
    scene(spec)
        .body(floor((0.0, 0.0)))
        .body(BodySpec::fixed(
            "wall",
            ShapeSpec::Polygon(vec![[-0.3, 0.0], [0.0, 0.0], [0.0, 1.0], [-0.3, 1.0]]),
        ))
        .body(
            BodySpec::rigid("block", m, ShapeSpec::Rectangle([0.6, 0.5]))
                .at(rest + amplitude, 0.25),
        )
        .constraint(ConstraintSpec::Slot {
            body: "block".into(),
            local: [0.0, 0.0],
            origin: [0.0, 0.25],
            direction: [1.0, 0.0],
        })
        .force(ForceSpec::Spring {
            a: AnchorSpec::Fixed([0.0, 0.25]),
            b: AnchorSpec::body_at("block", [-0.3, 0.0]),
            stiffness: k,
            rest_length: Some(rest - 0.3),
            pushes_only: false,
        })
        .focus("block")
        .answer("Period (s)", exact::spring_period(m, k))
        .answer("Maximum speed (m/s)", amplitude * (k / m).sqrt())
        .answer("Maximum force (N)", k * amplitude)
        .view([rest, 0.5], 2.2)
}

static PENDULUM: [Param; 2] = [
    param("length", "Length", "m", (0.2, 5.0), 0.1, 2.0),
    param("amplitude", "Release angle", "°", (1.0, 170.0), 1.0, 20.0),
];

fn simple_pendulum(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let (length, amplitude) = (p.get(spec, "length"), p.get(spec, "amplitude").to_radians());
    scene(spec)
        .body(
            ball("bob", 1.0, 0.08 * length).at(length * amplitude.sin(), -length * amplitude.cos()),
        )
        .constraint(ConstraintSpec::Rod {
            a: AnchorSpec::Fixed([0.0, 0.0]),
            b: AnchorSpec::body("bob"),
            length: Some(length),
        })
        .focus("bob")
        .answer(
            "Period, small-angle formula (s)",
            exact::pendulum_period_small(length, G),
        )
        .answer(
            "Period, exact (s)",
            exact::pendulum_period(length, G, amplitude),
        )
        .answer(
            "Speed at the bottom (m/s)",
            (2.0 * G * length * (1.0 - amplitude.cos())).sqrt(),
        )
        .view([0.0, -0.5 * length], 1.2 * length)
}

static DAMPED: [Param; 4] = [
    param("mass", "Mass", "kg", (0.1, 5.0), 0.1, 1.0),
    param(
        "stiffness",
        "Spring constant k",
        "N/m",
        (1.0, 100.0),
        1.0,
        20.0,
    ),
    param("damping", "Damping b", "kg/s", (0.0, 12.0), 0.1, 0.5),
    param(
        "amplitude",
        "Starting displacement",
        "m",
        (0.1, 1.5),
        0.05,
        0.8,
    ),
];

/// Where the block of an oscillator sits when its spring is relaxed.
const SPRING_REST: f64 = 2.0;

/// A block on a smooth track against a wall, joined to it by a spring: the body of every oscillator here.
fn oscillator(file: SceneFile, mass: f64, stiffness: f64, displacement: f64) -> SceneFile {
    file.body(floor((0.0, 0.0)))
        .body(BodySpec::fixed(
            "wall",
            ShapeSpec::Polygon(vec![[-0.3, 0.0], [0.0, 0.0], [0.0, 1.0], [-0.3, 1.0]]),
        ))
        .body(
            BodySpec::rigid("block", mass, ShapeSpec::Rectangle([0.6, 0.5]))
                .at(SPRING_REST + displacement, 0.25),
        )
        .constraint(ConstraintSpec::Slot {
            body: "block".into(),
            local: [0.0, 0.0],
            origin: [0.0, 0.25],
            direction: [1.0, 0.0],
        })
        .force(ForceSpec::Spring {
            a: AnchorSpec::Fixed([0.0, 0.25]),
            b: AnchorSpec::body_at("block", [-0.3, 0.0]),
            stiffness,
            rest_length: Some(SPRING_REST - 0.3),
            pushes_only: false,
        })
        .focus("block")
}

fn damped_oscillator(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let (m, k, b, amplitude) = (
        p.get(spec, "mass"),
        p.get(spec, "stiffness"),
        p.get(spec, "damping"),
        p.get(spec, "amplitude"),
    );
    let (gamma, omega0) = (b / (2.0 * m), (k / m).sqrt());
    let oscillates = gamma < omega0;
    let scene = oscillator(scene(spec), m, k, amplitude)
        .force(ForceSpec::LinearDrag {
            body: "block".into(),
            coefficient: b,
        })
        .answer("Oscillates? (1 = yes)", if oscillates { 1.0 } else { 0.0 })
        .answer("Period without damping (s)", exact::spring_period(m, k));
    let scene = if oscillates {
        scene.answer(
            "Period (s)",
            std::f64::consts::TAU / (omega0 * omega0 - gamma * gamma).sqrt(),
        )
    } else {
        scene
    };
    let scene = if b > 0.0 {
        scene.answer(
            "Time for the swing to halve (s)",
            std::f64::consts::LN_2 / gamma,
        )
    } else {
        scene
    };
    scene.view([SPRING_REST, 0.5], 2.2)
}

static DRIVEN: [Param; 5] = [
    param("mass", "Mass", "kg", (0.1, 5.0), 0.1, 1.0),
    param(
        "stiffness",
        "Spring constant k",
        "N/m",
        (1.0, 100.0),
        1.0,
        20.0,
    ),
    param("damping", "Damping b", "kg/s", (0.05, 5.0), 0.05, 0.5),
    param("force", "Driving force F₀", "N", (0.0, 10.0), 0.1, 2.0),
    param(
        "frequency",
        "Driving frequency ω",
        "rad/s",
        (0.5, 15.0),
        0.1,
        4.0,
    ),
];

fn driven_oscillator(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let (m, k, b, force, omega) = (
        p.get(spec, "mass"),
        p.get(spec, "stiffness"),
        p.get(spec, "damping"),
        p.get(spec, "force"),
        p.get(spec, "frequency"),
    );
    let (gamma, omega0) = (b / (2.0 * m), (k / m).sqrt());
    let (amplitude, lag) = exact::driven_response(m, k, b, force, omega);
    // The starting wobble dies as e^(−γt); six of those times, then a few steady cycles to look at.
    let duration = (6.0 / gamma + 5.0 * std::f64::consts::TAU / omega).clamp(10.0, 60.0);
    let scene = oscillator(
        SceneFile::new(spec.title, [0.0, -G], spec.dt).finish(FinishSpec::time(duration)),
        m,
        k,
        0.0,
    )
    .force(ForceSpec::LinearDrag {
        body: "block".into(),
        coefficient: b,
    })
    .force(ForceSpec::Driving {
        body: "block".into(),
        amplitude: [force, 0.0],
        angular_frequency: omega,
        phase: 0.0,
    })
    .answer("Natural frequency ω₀ (rad/s)", omega0)
    .answer("Steady amplitude (m)", amplitude)
    .answer("Lag behind the force (°)", lag.to_degrees());
    // The amplitude peaks a little below ω₀, and only if the damping is light enough for a peak at all.
    let scene = if omega0 * omega0 > 2.0 * gamma * gamma {
        scene
            .answer(
                "Frequency of the largest swing (rad/s)",
                (omega0 * omega0 - 2.0 * gamma * gamma).sqrt(),
            )
            .answer(
                "Largest steady amplitude (m)",
                force / (b * (omega0 * omega0 - gamma * gamma).sqrt()),
            )
    } else {
        scene
    };
    scene.frame([SPRING_REST, 0.5], (amplitude + 1.0).max(2.4), 2.2)
}

static COUPLED: [Param; 5] = [
    param("mass", "Mass of each block", "kg", (0.1, 5.0), 0.1, 1.0),
    param(
        "stiffness",
        "Outer springs k",
        "N/m",
        (1.0, 100.0),
        1.0,
        20.0,
    ),
    param("coupling", "Middle spring k′", "N/m", (0.0, 50.0), 0.5, 2.0),
    param("first", "Left block starts at", "m", (-0.5, 0.5), 0.05, 0.4),
    param(
        "second",
        "Right block starts at",
        "m",
        (-0.5, 0.5),
        0.05,
        0.0,
    ),
];

fn coupled_oscillators(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let (m, k, coupling, first, second) = (
        p.get(spec, "mass"),
        p.get(spec, "stiffness"),
        p.get(spec, "coupling"),
        p.get(spec, "first"),
        p.get(spec, "second"),
    );
    // Two blocks between two walls: wall, spring, block, spring, block, spring, wall. At rest the blocks sit at
    // x = 2 and x = 4 and no spring is stretched.
    let (left, right, far_wall, half) = (2.0, 4.0, 6.0, 0.3);
    let together = (k / m).sqrt();
    let against = ((k + 2.0 * coupling) / m).sqrt();
    let block = |id: &str, at: f64| {
        BodySpec::rigid(id, m, ShapeSpec::Rectangle([2.0 * half, 0.5])).at(at, 0.25)
    };
    let on_track = |id: &str| ConstraintSpec::Slot {
        body: id.into(),
        local: [0.0, 0.0],
        origin: [0.0, 0.25],
        direction: [1.0, 0.0],
    };
    let spring = |a: AnchorSpec, b: AnchorSpec, stiffness: f64, rest: f64| ForceSpec::Spring {
        a,
        b,
        stiffness,
        rest_length: Some(rest),
        pushes_only: false,
    };
    let wall = |id: &str, x: f64| {
        BodySpec::fixed(
            id,
            ShapeSpec::Polygon(vec![[x, 0.0], [x + 0.3, 0.0], [x + 0.3, 1.0], [x, 1.0]]),
        )
    };
    let scene = SceneFile::new(spec.title, [0.0, -G], spec.dt)
        .body(floor((0.0, 0.0)))
        .body(wall("left wall", -0.3))
        .body(wall("right wall", far_wall))
        .body(block("left block", left + first))
        .body(block("right block", right + second))
        .constraint(on_track("left block"))
        .constraint(on_track("right block"))
        .force(spring(
            AnchorSpec::Fixed([0.0, 0.25]),
            AnchorSpec::body_at("left block", [-half, 0.0]),
            k,
            left - half,
        ))
        .force(spring(
            AnchorSpec::body_at("left block", [half, 0.0]),
            AnchorSpec::body_at("right block", [-half, 0.0]),
            coupling,
            right - left - 2.0 * half,
        ))
        .force(spring(
            AnchorSpec::body_at("right block", [half, 0.0]),
            AnchorSpec::Fixed([far_wall, 0.25]),
            k,
            far_wall - right - half,
        ))
        .focus("left block")
        .answer(
            "Period moving together (s)",
            std::f64::consts::TAU / together,
        )
        .answer(
            "Period moving against each other (s)",
            std::f64::consts::TAU / against,
        );
    // Started any other way the motion is both at once, and it passes from one block to the other and back.
    let (scene, duration) = if coupling > 0.0 {
        let handover = std::f64::consts::PI / (against - together);
        (
            scene.answer("Time for the motion to cross over (s)", handover),
            (2.2 * handover).clamp(10.0, 40.0),
        )
    } else {
        (scene, 10.0)
    };
    scene
        .finish(FinishSpec::time(duration))
        .frame([0.5 * far_wall, 0.5], 0.5 * far_wall + 0.6, 2.2)
}

static VERTICAL_SPRING: [Param; 2] = [
    param("mass", "Mass", "kg", (0.1, 5.0), 0.1, 1.0),
    param(
        "stiffness",
        "Spring constant k",
        "N/m",
        (10.0, 200.0),
        1.0,
        40.0,
    ),
];

fn vertical_spring(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let (m, k) = (p.get(spec, "mass"), p.get(spec, "stiffness"));
    let stretch = m * G / k;
    let period = exact::spring_period(m, k);
    let (ceiling, natural) = (0.0, 1.0);
    let release = ceiling - natural;
    // Let go from where the spring is relaxed, the mass bobs between there and twice the stretch it would hang at.
    SceneFile::new(spec.title, [0.0, -G], spec.dt)
        .finish(FinishSpec::time(2.05 * period))
        .body(BodySpec::fixed(
            "ceiling",
            ShapeSpec::Polygon(vec![[-1.0, 0.0], [1.0, 0.0], [1.0, 0.2], [-1.0, 0.2]]),
        ))
        .body(ball("mass", m, 0.15).at(0.0, release))
        .constraint(ConstraintSpec::Slot {
            body: "mass".into(),
            local: [0.0, 0.0],
            origin: [0.0, 0.0],
            direction: [0.0, 1.0],
        })
        .force(ForceSpec::Spring {
            a: AnchorSpec::Fixed([0.0, ceiling]),
            b: AnchorSpec::body("mass"),
            stiffness: k,
            rest_length: Some(natural),
            pushes_only: false,
        })
        .focus("mass")
        .answer("Stretch it would hang at (m)", stretch)
        .answer("Lowest point below the release (m)", 2.0 * stretch)
        .answer("Period (s)", period)
        .answer("Greatest speed (m/s)", stretch * (k / m).sqrt())
        .view(
            [0.0, 0.5 * (0.4 + release - 2.0 * stretch - 0.4)],
            0.5 * (0.4 - (release - 2.0 * stretch - 0.4)),
        )
}

static TWO_SPRINGS: [Param; 4] = [
    param("k1", "First spring k₁", "N/m", (5.0, 200.0), 1.0, 30.0),
    param("k2", "Second spring k₂", "N/m", (5.0, 200.0), 1.0, 60.0),
    param("mass", "Mass of the block", "kg", (0.1, 5.0), 0.1, 1.0),
    param("amplitude", "Pulled out by", "m", (0.1, 1.0), 0.05, 0.5),
];

fn two_springs(spec: &ScenarioSpec, p: &Params) -> (f64, f64, f64, f64) {
    (
        p.get(spec, "k1"),
        p.get(spec, "k2"),
        p.get(spec, "mass"),
        p.get(spec, "amplitude"),
    )
}

/// A block on a smooth track to the right of a wall, its left face at `face`, for the two-spring scenes.
fn spring_block(file: SceneFile, mass: f64, face: f64) -> SceneFile {
    file.body(floor((0.0, 0.0)))
        .body(BodySpec::fixed(
            "wall",
            ShapeSpec::Polygon(vec![[-0.3, 0.0], [0.0, 0.0], [0.0, 1.0], [-0.3, 1.0]]),
        ))
        .body(BodySpec::rigid("block", mass, ShapeSpec::Rectangle([0.6, 0.5])).at(face + 0.3, 0.25))
        .constraint(ConstraintSpec::Slot {
            body: "block".into(),
            local: [0.0, 0.0],
            origin: [0.0, 0.25],
            direction: [1.0, 0.0],
        })
        .constraint(ConstraintSpec::FixedAngle {
            body: "block".into(),
            angle: 0.0,
        })
        .focus("block")
}

fn springs_in_series(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let (k1, k2, m, amplitude) = two_springs(spec, p);
    let effective = k1 * k2 / (k1 + k2);
    let period = exact::spring_period(m, effective);
    let natural = 1.5;
    // End to end both springs carry the same force, so the softer one stretches more. The join starts where
    // that already holds, and so has nothing to shake about.
    let first = amplitude * k2 / (k1 + k2);
    // The join between two springs has no mass in the textbook. Here it is a five-hundredth of the block,
    // and never so light that the springs would shake it faster than the time step can follow.
    let join = (0.002 * m).max(4.0 * (k1 + k2) * spec.dt * spec.dt);
    spring_block(
        SceneFile::new(spec.title, [0.0, -G], spec.dt).finish(FinishSpec::time(2.05 * period)),
        m,
        2.0 * natural + amplitude,
    )
    .body(ball("join", join, 0.06).at(natural + first, 0.25))
    .constraint(ConstraintSpec::Slot {
        body: "join".into(),
        local: [0.0, 0.0],
        origin: [0.0, 0.25],
        direction: [1.0, 0.0],
    })
    .force(ForceSpec::Spring {
        a: AnchorSpec::Fixed([0.0, 0.25]),
        b: AnchorSpec::body("join"),
        stiffness: k1,
        rest_length: Some(natural),
        pushes_only: false,
    })
    .force(ForceSpec::Spring {
        a: AnchorSpec::body("join"),
        b: AnchorSpec::body_at("block", [-0.3, 0.0]),
        stiffness: k2,
        rest_length: Some(natural),
        pushes_only: false,
    })
    .answer("Effective spring constant (N/m)", effective)
    .answer("Period (s)", period)
    .answer("Stretch of the first spring at the start (m)", first)
    .answer(
        "Stretch of the second spring at the start (m)",
        amplitude - first,
    )
    .frame([natural + 0.3, 0.6], natural + 1.6, 2.2)
}

fn springs_in_parallel(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let (k1, k2, m, amplitude) = two_springs(spec, p);
    let effective = k1 + k2;
    let period = exact::spring_period(m, effective);
    let natural = 2.0;
    // Side by side both springs stretch by the same amount, so their forces add.
    let side = |stiffness: f64, height: f64| ForceSpec::Spring {
        a: AnchorSpec::Fixed([0.0, 0.25 + height]),
        b: AnchorSpec::body_at("block", [-0.3, height]),
        stiffness,
        rest_length: Some(natural),
        pushes_only: false,
    };
    spring_block(
        SceneFile::new(spec.title, [0.0, -G], spec.dt).finish(FinishSpec::time(2.05 * period)),
        m,
        natural + amplitude,
    )
    .force(side(k1, 0.13))
    .force(side(k2, -0.13))
    .answer("Effective spring constant (N/m)", effective)
    .answer("Period (s)", period)
    .answer("Pull of the first spring at the start (N)", k1 * amplitude)
    .answer("Pull of the second spring at the start (N)", k2 * amplitude)
    .frame([natural, 0.6], natural + 0.8, 2.2)
}

// ---------------------------------------------------------------- Catalogue

#[allow(clippy::too_many_arguments)]
const fn spec(
    id: &'static str,
    title: &'static str,
    description: &'static str,
    chapter: &'static str,
    topic: &'static str,
    params: &'static [Param],
    dt: f64,
    duration: f64,
) -> ScenarioSpec {
    ScenarioSpec {
        id,
        title,
        description,
        syllabus: Syllabus {
            class: 11,
            chapter,
            topic,
        },
        params,
        dt,
        duration,
    }
}

static CATALOGUE: &[Scenario] = &[
    Scenario {
        spec: spec("free-fall", "Free fall", "A ball dropped from rest. Watch the v–t graph: a straight line whose slope is g.", "Motion in a straight line", "uniform acceleration", &FREE_FALL, 1e-3, 4.0),
        describe: free_fall,
    },
    Scenario {
        spec: spec("thrown-upward", "Ball thrown upward", "The ball slows, stops for an instant at the top, and falls back. The acceleration never changes.", "Motion in a straight line", "uniform acceleration", &THROWN_UP, 1e-3, 4.0),
        describe: thrown_upward,
    },
    Scenario {
        spec: spec("stopping-distance", "Stopping distance", "A block sliding on a rough floor. Double the speed and it travels four times as far before it stops.", "Motion in a straight line", "uniform retardation", &STOPPING, 1e-3, 4.0),
        describe: stopping_distance,
    },
    Scenario {
        spec: spec("projectile", "Projectile", "Horizontal motion at constant speed, vertical motion under gravity, together a parabola.", "Motion in a plane", "projectile motion", &PROJECTILE, 1e-3, 5.0),
        describe: projectile,
    },
    Scenario {
        spec: spec("projectile-from-height", "Projectile from a cliff", "Launched from a height, even horizontally. Try a negative angle.", "Motion in a plane", "projectile motion", &PROJECTILE_HEIGHT, 1e-3, 5.0),
        describe: projectile_from_height,
    },
    Scenario {
        spec: spec("projectile-on-incline", "Projectile on an incline", "Thrown up a slope, the ball lands sooner than it would on level ground. The longest throw is aimed halfway between the slope and straight up.", "Motion in a plane", "projectile motion", &ON_INCLINE, 1e-3, 5.0),
        describe: projectile_on_incline,
    },
    Scenario {
        spec: spec("circular-motion", "Uniform circular motion", "A stone whirled on a string, seen from above. Its speed never changes, yet it is accelerating all the time: the string keeps pulling it off the straight line it would otherwise take.", "Motion in a plane", "uniform circular motion", &CIRCULAR_MOTION, 1e-3, 4.0),
        describe: circular_motion,
    },
    Scenario {
        spec: spec("incline-friction", "Block on a rough incline", "Static friction holds the block until tan θ exceeds μs. Then kinetic friction takes over and it accelerates.", "Laws of motion", "friction", &INCLINE, 1e-3, 3.0),
        describe: incline_friction,
    },
    Scenario {
        spec: spec("pushed-block", "Pushing a block on a rough floor", "Below μs N the block does not move and friction equals your push exactly. Above it, kinetic friction is smaller and the block accelerates.", "Laws of motion", "friction", &PUSHED, 1e-3, 3.0),
        describe: pushed_block,
    },
    Scenario {
        spec: spec("connected-blocks", "Two blocks joined by a string", "Pull the front block and the string drags the other along. The string only has to accelerate the block behind it, so its tension is a fixed share of your pull.", "Laws of motion", "connected bodies", &CONNECTED, 1e-3, 3.0),
        describe: connected_blocks,
    },
    Scenario {
        spec: spec("lift", "Apparent weight in a lift", "You feel heavier when a lift starts upward and lighter when it starts down. A scale reads the push of the floor, not the pull of the Earth.", "Laws of motion", "apparent weight", &LIFT, 1e-3, 3.0),
        describe: lift,
    },
    Scenario {
        spec: spec("recoil", "Recoil of a gun", "Shell and gun push each other apart with equal forces for the same time, so they leave with equal and opposite momentum. The light one gets nearly all the energy.", "Laws of motion", "conservation of momentum", &RECOIL, 1e-3, 2.0),
        describe: recoil,
    },
    Scenario {
        spec: spec("atwood", "Atwood machine", "Two masses on a string over a massless pulley. The heavier side descends with the same acceleration the lighter side rises.", "Laws of motion", "connected bodies", &ATWOOD, 1e-3, 2.0),
        describe: atwood,
    },
    Scenario {
        spec: spec("table-and-hanging-mass", "Block on a table and a hanging mass", "The hanging mass pulls the block across the table. Add table friction and see when it refuses to move.", "Laws of motion", "connected bodies", &TABLE, 1e-3, 2.0),
        describe: table_and_hanging_mass,
    },
    Scenario {
        spec: spec("collision-1d", "Head-on collision", "Momentum is always conserved. Kinetic energy is conserved only when e = 1.", "Work, energy and power", "collisions", &COLLISION, 1e-3, 4.0),
        describe: collision_1d,
    },
    Scenario {
        spec: spec("vertical-circle", "Bob in a vertical circle", "The string goes slack if the bob is too slow at the top. Find the speed at the bottom that just completes the circle.", "Work, energy and power", "energy in circular motion", &VERTICAL_CIRCLE, 1e-4, 4.0),
        describe: vertical_circle,
    },
    Scenario {
        spec: spec("spring-compression", "A block runs into a spring", "The block's kinetic energy goes into the spring until none is left, then the spring gives all of it back.", "Work, energy and power", "elastic potential energy", &SPRING_STOP, 1e-3, 5.0),
        describe: spring_compression,
    },
    Scenario {
        spec: spec("ball-on-spring", "A ball dropped onto a spring", "The spring is squeezed further than the ball's weight alone would squeeze it: the ball arrives moving, and keeps losing height while the spring slows it.", "Work, energy and power", "conservation of mechanical energy", &DROP_SPRING, 1e-3, 4.0),
        describe: ball_on_spring,
    },
    Scenario {
        spec: spec("oblique-collision", "A glancing collision", "Two equal discs on a smooth table, seen from above. However off-centre the hit, they leave at right angles to each other.", "Work, energy and power", "collisions in two dimensions", &OBLIQUE, 1e-3, 6.0),
        describe: oblique_collision,
    },
    Scenario {
        spec: spec("rolling", "Rolling down an incline", "Friction makes it roll instead of slide, but only if there is enough of it. Compare a disc and a ring.", "System of particles and rotational motion", "rolling motion", &ROLLING, 1e-3, 3.0),
        describe: rolling,
    },
    Scenario {
        spec: spec("atwood-massive", "Atwood machine with a massive pulley", "Now the pulley has to be spun up too. The tensions on the two sides differ, and that difference is what turns the pulley.", "System of particles and rotational motion", "rotational dynamics", &ATWOOD_MASSIVE, 1e-3, 2.0),
        describe: atwood_massive,
    },
    Scenario {
        spec: spec("flywheel", "Flywheel with a hanging mass", "A string wound on a heavy wheel is pulled by a falling mass. The heavier the wheel, the slower the mass falls.", "System of particles and rotational motion", "rotational dynamics", &FLYWHEEL, 1e-3, 3.0),
        describe: flywheel,
    },
    Scenario {
        spec: spec("seesaw", "Seesaw", "Balance needs equal torques, not equal masses. Move a mass outward and watch the plank tip.", "System of particles and rotational motion", "torque and equilibrium", &SEESAW, 1e-3, 3.0),
        describe: seesaw,
    },
    Scenario {
        spec: spec("torque-on-rod", "Torque on a pivoted rod", "A steady twist on a rod pivoted at one end, seen from above. It speeds up at a constant rate, and how fast depends on where its mass is, not only how much there is.", "System of particles and rotational motion", "rotational dynamics", &TORQUE_ROD, 1e-3, 2.0),
        describe: torque_on_rod,
    },
    Scenario {
        spec: spec("circular-orbit", "Satellite in a circular orbit", "Fast enough sideways and falling becomes going round. The higher the orbit, the slower the satellite and the longer its year.", "Gravitation", "orbits", &CIRCULAR_ORBIT, 1.0, 11000.0),
        describe: circular_orbit,
    },
    Scenario {
        spec: spec("elliptical-orbit", "Elliptical orbit", "Give the satellite more than circular speed and the circle stretches into an ellipse. It races past the near point and crawls past the far one, sweeping equal areas in equal times.", "Gravitation", "Kepler's laws", &ELLIPTICAL_ORBIT, 2.0, 22000.0),
        describe: elliptical_orbit,
    },
    Scenario {
        spec: spec("escape-velocity", "Escape velocity", "Throw something straight up and it comes back, unless it leaves faster than 11.2 km/s. Then gravity slows it for ever and never stops it.", "Gravitation", "escape speed", &ESCAPE, 1.0, 10000.0),
        describe: escape_velocity,
    },
    Scenario {
        spec: spec("spring-mass", "Mass on a spring", "Simple harmonic motion. The period depends on m and k, not on the amplitude.", "Oscillations", "simple harmonic motion", &SPRING_MASS, 1e-3, 6.0),
        describe: spring_mass,
    },
    Scenario {
        spec: spec("vertical-spring", "A mass on a vertical spring", "Gravity moves the middle of the swing down, to where the mass would hang at rest, and changes nothing else: the period is the same as on a level table.", "Oscillations", "simple harmonic motion", &VERTICAL_SPRING, 1e-3, 4.0),
        describe: vertical_spring,
    },
    Scenario {
        spec: spec("springs-in-series", "Springs in series", "Two springs end to end are softer than either alone: each one stretches, and the stretches add.", "Oscillations", "combinations of springs", &TWO_SPRINGS, 1e-3, 4.0),
        describe: springs_in_series,
    },
    Scenario {
        spec: spec("springs-in-parallel", "Springs in parallel", "Two springs side by side are stiffer than either alone: they stretch together, and their pulls add.", "Oscillations", "combinations of springs", &TWO_SPRINGS, 1e-3, 4.0),
        describe: springs_in_parallel,
    },
    Scenario {
        spec: spec("simple-pendulum", "Simple pendulum", "For small swings the period is 2π√(L/g). Release it from a large angle and see the period grow.", "Oscillations", "pendulum", &PENDULUM, 1e-3, 8.0),
        describe: simple_pendulum,
    },
    Scenario {
        spec: spec("damped-oscillator", "Damped oscillator", "Friction that grows with speed steals a fixed fraction of the swing each cycle. Add enough and it stops oscillating at all.", "Oscillations", "damped oscillations", &DAMPED, 1e-3, 10.0),
        describe: damped_oscillator,
    },
    Scenario {
        spec: spec("driven-oscillator", "Driven oscillator and resonance", "Push in rhythm and the block settles into your rhythm, not its own. Push at its own frequency and the swing grows large: resonance.", "Oscillations", "forced oscillations and resonance", &DRIVEN, 1e-3, 30.0),
        describe: driven_oscillator,
    },
    Scenario {
        spec: spec("coupled-oscillators", "Coupled oscillators", "Two blocks joined by a weak spring. Start one and its motion drains into the other and comes back. Start them together, or exactly against each other, and it never does.", "Oscillations", "normal modes and beats", &COUPLED, 1e-3, 16.0),
        describe: coupled_oscillators,
    },
];
