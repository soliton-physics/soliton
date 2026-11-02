//! The scenario catalogue (design 004): named, parameterised scenes in NCERT chapter order, each with its textbook
//! answers. Every scenario is produced as a [`SceneFile`], so the JSON a teacher can export is exactly what runs.

use crate::body::BodyId;
use crate::exact;
use crate::scene::{
    AnchorSpec, BodySpec, ConstraintSpec, ForceSpec, RopeNodeSpec, SceneFile, ShapeSpec, WrapSpec,
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

fn scene(spec: &ScenarioSpec) -> SceneFile {
    SceneFile::new(spec.title, [0.0, -G], spec.dt)
}

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
                .at(0.0, 0.3)
                .velocity(0.0, u)
                .restitution(0.0),
        )
        .focus("ball")
        .answer("Maximum height (m)", apex)
        .answer("Time to the top (s)", u / G)
        .answer("Time of flight (s)", 2.0 * u / G)
        .view([0.0, 0.5 * apex], 0.6 * apex + 1.0)
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
                .at(0.0, 0.25)
                .velocity(u * angle.cos(), u * angle.sin())
                .restitution(0.0),
        )
        .focus("ball")
        .answer("Range (m)", range)
        .answer("Maximum height (m)", height)
        .answer(
            "Time of flight (s)",
            exact::projectile_flight_time(u, angle, G),
        )
        .view(
            [0.5 * range, 0.5 * height],
            0.5 * range.max(height) * 1.3 + 1.0,
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
                .at(0.0, h + 0.25)
                .velocity(ux, uy)
                .restitution(0.0),
        )
        .focus("ball")
        .answer("Horizontal range (m)", range)
        .answer("Time of flight (s)", flight)
        .answer("Speed on landing (m/s)", (u * u + 2.0 * G * h).sqrt())
        .view([0.5 * range, 0.5 * h], 0.5 * range.max(h) * 1.3 + 1.0)
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

static ATWOOD: [Param; 2] = [
    param("m1", "Left mass", "kg", (0.1, 10.0), 0.1, 2.0),
    param("m2", "Right mass", "kg", (0.1, 10.0), 0.1, 3.0),
];

fn atwood(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let (m1, m2) = (p.get(spec, "m1"), p.get(spec, "m2"));
    let radius = 0.4;
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
        .body(ball("m1", m1, 0.2).at(-radius, -5.0).restitution(0.0))
        .body(ball("m2", m2, 0.2).at(radius, -5.0).restitution(0.0))
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
        .answer(
            "Acceleration (m/s²)",
            exact::atwood_acceleration(m1, m2, G).abs(),
        )
        .answer("Tension (N)", exact::atwood_tension(m1, m2, G))
        .view([0.0, -3.8], 4.3)
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
        .answer("Moves? (1 = yes)", if moves { 1.0 } else { 0.0 })
        .answer("Acceleration (m/s²)", a)
        .answer(
            "Tension (N)",
            if moves { m1 * (a + mu * G) } else { m2 * G },
        )
        .answer("Normal force on block (N)", m1 * G)
        .view([1.0, -0.8], 3.0)
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
        .answer("Left torque m1 d1 g (N·m)", m1 * d1 * G)
        .answer("Right torque m2 d2 g (N·m)", m2 * d2 * G)
        .answer(
            "Initial angular acceleration (rad/s²)",
            exact::seesaw_angular_acceleration(m1, d1, m2, d2, inertia, G),
        )
        .view([0.0, 1.0], 2.5)
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
        spec: spec("projectile", "Projectile", "Horizontal motion at constant speed, vertical motion under gravity, together a parabola.", "Motion in a plane", "projectile motion", &PROJECTILE, 1e-3, 5.0),
        describe: projectile,
    },
    Scenario {
        spec: spec("projectile-from-height", "Projectile from a cliff", "Launched from a height, even horizontally. Try a negative angle.", "Motion in a plane", "projectile motion", &PROJECTILE_HEIGHT, 1e-3, 5.0),
        describe: projectile_from_height,
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
        spec: spec("spring-mass", "Mass on a spring", "Simple harmonic motion. The period depends on m and k, not on the amplitude.", "Oscillations", "simple harmonic motion", &SPRING_MASS, 1e-3, 6.0),
        describe: spring_mass,
    },
    Scenario {
        spec: spec("simple-pendulum", "Simple pendulum", "For small swings the period is 2π√(L/g). Release it from a large angle and see the period grow.", "Oscillations", "pendulum", &PENDULUM, 1e-3, 8.0),
        describe: simple_pendulum,
    },
];
