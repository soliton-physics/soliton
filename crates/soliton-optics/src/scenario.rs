//! The catalogue: NCERT class 12, ray optics and optical instruments (design 013). Each scenario is a function
//! from a few parameters to a scene, with the textbook's answer beside what the traced rays give.
//!
//! Distances follow the textbook's sign convention: light travels to the right, and distances are measured from
//! the pole of a mirror or the centre of a lens, positive to the right and upward.

use crate::curve::Curve;
use crate::scene::{MeasureSpec, OpticsScene, SourceSpec, SyllabusSpec};
use serde::Serialize;
use soliton_core::Vec2;

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

#[derive(Clone, Copy, Debug, Serialize)]
pub struct ScenarioSpec {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub syllabus: Syllabus,
    pub params: &'static [Param],
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

    /// The value of a parameter, kept within the range the scenario allows.
    fn get(&self, spec: &ScenarioSpec, name: &str) -> f64 {
        let param = spec
            .params
            .iter()
            .find(|p| p.name == name)
            .unwrap_or_else(|| panic!("scenario `{}` has no parameter `{name}`", spec.id));
        self.values
            .iter()
            .find(|(n, _)| n == name)
            .map_or(param.default, |(_, v)| v.clamp(param.min, param.max))
    }
}

pub struct Scenario {
    pub spec: ScenarioSpec,
    describe: fn(&ScenarioSpec, &Params) -> OpticsScene,
}

impl Scenario {
    pub fn scene(&self, params: &Params) -> OpticsScene {
        let spec = &self.spec;
        let mut scene = (self.describe)(spec, params);
        scene.description = Some(spec.description.to_string());
        scene.syllabus = Some(SyllabusSpec {
            class: spec.syllabus.class,
            chapter: spec.syllabus.chapter.to_string(),
            topic: spec.syllabus.topic.to_string(),
        });
        scene
    }
}

pub fn catalogue() -> &'static [Scenario] {
    CATALOGUE
}

pub fn find(id: &str) -> Option<&'static Scenario> {
    CATALOGUE.iter().find(|s| s.spec.id == id)
}

const CM: f64 = 0.01;
const PER_M: f64 = 100.0;
/// A length large enough to stand for "goes on for ever" in a drawing a metre or so across.
const VAST: f64 = 50.0;

fn rectangle(min: [f64; 2], max: [f64; 2]) -> Vec<Curve> {
    let corners = [
        Vec2::new(min[0], min[1]),
        Vec2::new(max[0], min[1]),
        Vec2::new(max[0], max[1]),
        Vec2::new(min[0], max[1]),
    ];
    (0..4)
        .map(|i| Curve::Segment {
            a: corners[i],
            b: corners[(i + 1) % 4],
        })
        .collect()
}

/// Everything below the x axis filled with one material: a pond, a block of glass.
fn below(scene: OpticsScene, index: f64) -> OpticsScene {
    scene.glass(rectangle([-VAST, -VAST], [VAST, 0.0]), index)
}

fn image_distance(object: usize) -> MeasureSpec {
    MeasureSpec::ImagePosition {
        object,
        origin: [0.0, 0.0],
        axis: [1.0, 0.0],
        scale: PER_M,
    }
}

// ---------------------------------------------------------------- spherical mirrors

static CONCAVE_MIRROR: [Param; 3] = [
    param("focal", "Focal length", "cm", (5.0, 30.0), 1.0, 15.0),
    param("distance", "Object distance", "cm", (4.0, 90.0), 1.0, 40.0),
    param("height", "Object height", "cm", (1.0, 5.0), 0.5, 3.0),
];

static CONVEX_MIRROR: [Param; 3] = [
    param("focal", "Focal length", "cm", (5.0, 30.0), 1.0, 15.0),
    param("distance", "Object distance", "cm", (4.0, 90.0), 1.0, 30.0),
    param("height", "Object height", "cm", (1.0, 5.0), 0.5, 3.0),
];

/// A spherical mirror with its pole at the origin, the object to its left. `focal` carries the textbook's sign:
/// negative for a concave mirror.
fn mirror(spec: &ScenarioSpec, p: &Params, converging: bool) -> OpticsScene {
    let size = p.get(spec, "focal") * CM;
    let (distance, height) = (p.get(spec, "distance") * CM, p.get(spec, "height") * CM);
    let radius = 2.0 * size;
    let (f, u) = (if converging { -size } else { size }, -distance);
    // 1/v + 1/u = 1/f
    let v = u * f / (u - f);
    let m = -v / u;
    let aperture = (0.55 * radius).min((2.0 * height).max(0.3 * radius));
    let towards_centre = if converging { -1.0 } else { 1.0 };
    let centre = [towards_centre * radius, 0.0];
    let tip = [u, height];
    let left = (u.min(v) - 0.05).min(-radius - 0.05);
    // Behind a concave mirror there is nothing to show unless the image is there.
    let right = if converging {
        0.08_f64.max(v.min(0.6) + 0.05)
    } else {
        radius + 0.05
    };
    OpticsScene::new(spec.title)
        .mirror(Curve::cap(
            Vec2::ZERO,
            Vec2::new(towards_centre, 0.0),
            radius,
            aperture,
        ))
        .axis([left - 1.0, 0.0], [right + 1.0, 0.0])
        .mark([0.0, 0.0], "P")
        .mark([f, 0.0], "F")
        .mark(centre, "C")
        .object([u, 0.0], tip, [0.0, 0.0])
        // The three rays whose reflections can be drawn with a ruler: parallel to the axis, towards the centre
        // of curvature, and to the pole.
        .ray(tip, [0.0, height])
        .ray(tip, centre)
        .ray(tip, [0.0, 0.0])
        .measured("Image distance, v (cm)", v * PER_M, image_distance(0))
        .measured(
            "Magnification, m = −v/u",
            m,
            MeasureSpec::Magnification { object: 0 },
        )
        .answer("Image height (cm)", m * height * PER_M)
        .answer("Real image? (1 = yes)", f64::from(v < 0.0))
        .view(
            [0.5 * (left + right), 0.0],
            0.5 * (right - left),
            (1.3 * aperture).max(1.4 * (m * height).abs().min(4.0 * aperture)),
        )
}

fn concave_mirror(spec: &ScenarioSpec, p: &Params) -> OpticsScene {
    mirror(spec, p, true)
}

fn convex_mirror(spec: &ScenarioSpec, p: &Params) -> OpticsScene {
    mirror(spec, p, false)
}

// ---------------------------------------------------------------- refraction at flat surfaces

static SNELL: [Param; 3] = [
    param("angle", "Angle of incidence", "°", (0.0, 89.0), 1.0, 45.0),
    param("above", "Refractive index above", "", (1.0, 2.4), 0.01, 1.0),
    param(
        "below",
        "Refractive index below",
        "",
        (1.0, 2.4),
        0.01,
        1.33,
    ),
];

fn snells_law(spec: &ScenarioSpec, p: &Params) -> OpticsScene {
    let i = p.get(spec, "angle").to_radians();
    let (above, under) = (p.get(spec, "above"), p.get(spec, "below"));
    let sine = above * i.sin() / under;
    let from = [-0.3 * i.sin(), 0.3 * i.cos()];
    let mut scene = below(OpticsScene::new(spec.title), under);
    scene.ambient = above;
    scene.reach = 0.6;
    scene = scene
        .source(SourceSpec::Ray {
            from,
            through: [0.0, 0.0],
            angles: true,
            faint: false,
        })
        .view([0.0, 0.0], 0.42, 0.34);
    if sine <= 1.0 {
        scene = scene
            .measured(
                "Angle of refraction (°)",
                sine.asin().to_degrees(),
                MeasureSpec::Emergence { ray: 0, event: 0 },
            )
            .measured(
                "Turned through (°)",
                (i - sine.asin()).abs().to_degrees(),
                MeasureSpec::Deviation { ray: 0 },
            );
    }
    if above > under {
        scene = scene
            .answer("Critical angle (°)", (under / above).asin().to_degrees())
            .answer("Totally reflected? (1 = yes)", f64::from(sine > 1.0));
    }
    scene.answer("Speed below, as a share of the speed above", above / under)
}

static TOTAL_REFLECTION: [Param; 2] = [
    param(
        "angle",
        "Angle of incidence, in the water",
        "°",
        (0.0, 89.0),
        1.0,
        35.0,
    ),
    param(
        "index",
        "Refractive index of the water",
        "",
        (1.1, 2.4),
        0.01,
        1.33,
    ),
];

fn total_internal_reflection(spec: &ScenarioSpec, p: &Params) -> OpticsScene {
    let (i, n) = (p.get(spec, "angle").to_radians(), p.get(spec, "index"));
    let depth = 0.2;
    let lamp = [0.0, -depth];
    let sine = n * i.sin();
    let mut scene = below(OpticsScene::new(spec.title), n)
        .source(SourceSpec::Ray {
            from: lamp,
            through: [depth * i.tan(), 0.0],
            angles: true,
            faint: false,
        })
        // A faint spray of rays from the same lamp: only those inside a cone get out.
        .source(SourceSpec::Fan {
            from: lamp,
            towards: [0.0, 0.0],
            half_angle: 80f64.to_radians(),
            count: 17,
            faint: true,
        })
        .mark(lamp, "lamp")
        .answer(
            "Critical angle, sin⁻¹(1/n) (°)",
            (1.0 / n).asin().to_degrees(),
        )
        .answer("Totally reflected? (1 = yes)", f64::from(sine > 1.0))
        .answer(
            "Radius of the bright circle on the surface (cm)",
            depth / (n * n - 1.0).sqrt() * PER_M,
        )
        .view([0.0, -0.03], 0.5, 0.3);
    scene.reach = 1.5;
    if sine <= 1.0 {
        scene = scene.measured(
            "Angle in the air (°)",
            sine.asin().to_degrees(),
            MeasureSpec::Emergence { ray: 0, event: 0 },
        );
    }
    scene
}

static APPARENT_DEPTH: [Param; 2] = [
    param("depth", "Real depth", "cm", (5.0, 40.0), 1.0, 20.0),
    param(
        "index",
        "Refractive index of the water",
        "",
        (1.1, 2.4),
        0.01,
        1.33,
    ),
];

fn apparent_depth(spec: &ScenarioSpec, p: &Params) -> OpticsScene {
    let (depth, n) = (p.get(spec, "depth") * CM, p.get(spec, "index"));
    let coin = [0.0, -depth];
    let mut scene = below(OpticsScene::new(spec.title), n)
        .source(SourceSpec::Fan {
            from: coin,
            towards: [0.0, 0.0],
            half_angle: 12f64.to_radians(),
            count: 5,
            faint: false,
        })
        .object(coin, [0.02, -depth], [0.0, 0.0])
        .measured(
            "Apparent depth, d/n (cm)",
            depth / n * PER_M,
            MeasureSpec::ImagePosition {
                object: 0,
                origin: [0.0, 0.0],
                axis: [0.0, -1.0],
                scale: PER_M,
            },
        )
        .answer("Raised by (cm)", depth * (1.0 - 1.0 / n) * PER_M)
        .view([0.0, -0.4 * depth], 0.9 * depth + 0.1, 0.75 * depth + 0.06);
    scene.reach = depth + 0.3;
    scene
}

static SLAB: [Param; 3] = [
    param("angle", "Angle of incidence", "°", (0.0, 85.0), 1.0, 50.0),
    param(
        "thickness",
        "Thickness of the slab",
        "cm",
        (2.0, 20.0),
        0.5,
        10.0,
    ),
    param(
        "index",
        "Refractive index of the glass",
        "",
        (1.1, 2.4),
        0.01,
        1.5,
    ),
];

fn glass_slab(spec: &ScenarioSpec, p: &Params) -> OpticsScene {
    let i = p.get(spec, "angle").to_radians();
    let (t, n) = (p.get(spec, "thickness") * CM, p.get(spec, "index"));
    let r = (i.sin() / n).asin();
    let mut scene = OpticsScene::new(spec.title)
        .glass(rectangle([-VAST, -t], [VAST, 0.0]), n)
        .source(SourceSpec::Ray {
            from: [-0.15 * i.sin(), 0.15 * i.cos()],
            through: [0.0, 0.0],
            angles: true,
            faint: false,
        })
        .measured(
            "Angle inside the glass (°)",
            r.to_degrees(),
            MeasureSpec::Emergence { ray: 0, event: 0 },
        )
        .measured(
            "Angle it comes out at (°)",
            i.to_degrees(),
            MeasureSpec::Emergence { ray: 0, event: 1 },
        )
        .measured(
            "Turned through in all (°)",
            0.0,
            MeasureSpec::Deviation { ray: 0 },
        )
        .answer(
            "Shifted sideways by, t sin(i − r)/cos r (cm)",
            t * (i - r).sin() / r.cos() * PER_M,
        )
        .view([0.4 * t * r.tan(), -0.5 * t], 0.3, 0.6 * t + 0.14);
    scene.reach = 0.6;
    scene
}

// ---------------------------------------------------------------- prism

static PRISM: [Param; 3] = [
    param(
        "apex",
        "Angle of the prism, A",
        "°",
        (30.0, 75.0),
        1.0,
        60.0,
    ),
    param("index", "Refractive index", "", (1.3, 1.9), 0.01, 1.5),
    param("angle", "Angle of incidence", "°", (20.0, 85.0), 1.0, 49.0),
];

fn prism(spec: &ScenarioSpec, p: &Params) -> OpticsScene {
    let (apex, n) = (p.get(spec, "apex").to_radians(), p.get(spec, "index"));
    let i = p.get(spec, "angle").to_radians();
    let height = 0.12;
    let half_base = height * (0.5 * apex).tan();
    let corners = [
        Vec2::new(-half_base, 0.0),
        Vec2::new(half_base, 0.0),
        Vec2::new(0.0, height),
    ];
    let outline: Vec<Curve> = (0..3)
        .map(|k| Curve::Segment {
            a: corners[k],
            b: corners[(k + 1) % 3],
        })
        .collect();
    // The ray meets the left face at its middle, at `i` to the normal there.
    let meet = (corners[0] + corners[2]) * 0.5;
    let direction = Vec2::from_polar(1.0, -0.5 * apex + i);
    let start = meet - direction * 0.16;
    let r1 = (i.sin() / n).asin();
    let r2 = apex - r1;
    let out = n * r2.sin();
    // D_m = 2 sin⁻¹(n sin A/2) − A, when the ray inside runs parallel to the base.
    let least = 2.0 * (n * (0.5 * apex).sin()).min(1.0).asin() - apex;
    let mut scene = OpticsScene::new(spec.title)
        .glass(outline, n)
        .source(SourceSpec::Ray {
            from: [start.x, start.y],
            through: [meet.x, meet.y],
            angles: true,
            faint: false,
        })
        .view([0.02, 0.5 * height], 0.22, 0.11);
    scene.reach = 0.3;
    if out <= 1.0 && r2 >= 0.0 {
        scene = scene
            .measured(
                "Angle of emergence, e (°)",
                out.asin().to_degrees(),
                MeasureSpec::Emergence { ray: 0, event: 1 },
            )
            .measured(
                "Deviation, i + e − A (°)",
                (i + out.asin() - apex).to_degrees(),
                MeasureSpec::Deviation { ray: 0 },
            );
    }
    scene
        .answer(
            "Comes out of the second face? (1 = yes)",
            f64::from(out <= 1.0),
        )
        .answer("Least deviation there can be, Dₘ (°)", least.to_degrees())
        .answer(
            "Angle of incidence that gives it, (A + Dₘ)/2 (°)",
            (0.5 * (apex + least)).to_degrees(),
        )
}

// ---------------------------------------------------------------- a single spherical surface

static SURFACE: [Param; 3] = [
    param(
        "radius",
        "Radius of the surface",
        "cm",
        (5.0, 30.0),
        1.0,
        10.0,
    ),
    param(
        "index",
        "Refractive index of the glass",
        "",
        (1.1, 2.4),
        0.01,
        1.5,
    ),
    param("distance", "Object distance", "cm", (5.0, 100.0), 1.0, 40.0),
];

fn spherical_surface(spec: &ScenarioSpec, p: &Params) -> OpticsScene {
    let (radius, n) = (p.get(spec, "radius") * CM, p.get(spec, "index"));
    let u = -p.get(spec, "distance") * CM;
    // n₂/v − n₁/u = (n₂ − n₁)/R, from air into the glass.
    let v = n / ((n - 1.0) / radius + 1.0 / u);
    let aperture = 0.6 * radius;
    let face = Curve::cap(Vec2::ZERO, Vec2::new(1.0, 0.0), radius, aperture);
    let edge = face.start().x;
    let mut outline = vec![face];
    for (a, b) in [
        ([edge, aperture], [VAST, aperture]),
        ([VAST, aperture], [VAST, -aperture]),
        ([VAST, -aperture], [edge, -aperture]),
    ] {
        outline.push(Curve::Segment {
            a: Vec2::new(a[0], a[1]),
            b: Vec2::new(b[0], b[1]),
        });
    }
    let shown = if v.is_finite() {
        v.clamp(-1.5, 1.5)
    } else {
        0.5
    };
    let (left, right) = (u.min(shown) - 0.05, shown.max(2.0 * radius) + 0.08);
    let mut scene = OpticsScene::new(spec.title)
        .glass(outline, n)
        .axis([left - 1.0, 0.0], [right + 1.0, 0.0])
        .mark([0.0, 0.0], "P")
        .mark([radius, 0.0], "C")
        .source(SourceSpec::Fan {
            from: [u, 0.0],
            towards: [0.0, 0.0],
            half_angle: (0.5 * aperture / u.abs()).atan(),
            count: 5,
            faint: false,
        })
        .object([u, 0.0], [u, 0.01], [0.0, 0.0])
        .measured("Image distance, v (cm)", v * PER_M, image_distance(0))
        .answer("Real image? (1 = yes)", f64::from(v > 0.0))
        .answer(
            "Object distance that sends the light on parallel (cm)",
            radius / (n - 1.0) * PER_M,
        )
        .view(
            [0.5 * (left + right), 0.0],
            0.5 * (right - left),
            1.6 * aperture,
        );
    scene.reach = right + 1.0;
    scene
}

// ---------------------------------------------------------------- thin lenses

static CONVEX_LENS: [Param; 3] = [
    param("focal", "Focal length", "cm", (5.0, 30.0), 1.0, 15.0),
    param("distance", "Object distance", "cm", (4.0, 90.0), 1.0, 40.0),
    param("height", "Object height", "cm", (1.0, 5.0), 0.5, 3.0),
];

static CONCAVE_LENS: [Param; 3] = [
    param("focal", "Focal length", "cm", (5.0, 30.0), 1.0, 15.0),
    param("distance", "Object distance", "cm", (4.0, 90.0), 1.0, 30.0),
    param("height", "Object height", "cm", (1.0, 5.0), 0.5, 3.0),
];

/// An ideal thin lens at the origin with the object to its left; `focal` is negative for a diverging lens.
fn lens(spec: &ScenarioSpec, p: &Params, converging: bool) -> OpticsScene {
    let size = p.get(spec, "focal") * CM;
    let (distance, height) = (p.get(spec, "distance") * CM, p.get(spec, "height") * CM);
    let (f, u) = (if converging { size } else { -size }, -distance);
    // 1/v − 1/u = 1/f
    let v = u * f / (u + f);
    let m = v / u;
    let tip = [u, height];
    // The ray that leaves parallel to the axis: the one along the line through the first focus, at −f.
    let slope = height / (u + f);
    let shown = if v.is_finite() {
        v.clamp(-1.5, 1.5)
    } else {
        0.5
    };
    let image_height = (m * height).abs().min(0.3);
    let aperture = (1.6 * height).max(1.25 * image_height.min(0.12)).max(0.04);
    let left = u.min(shown).min(-2.0 * size) - 0.05;
    let right = shown.max(2.0 * size) + 0.05;
    OpticsScene::new(spec.title)
        .thin_lens(0.0, f, aperture)
        .axis([left - 1.0, 0.0], [right + 1.0, 0.0])
        .mark([0.0, 0.0], "O")
        .mark([-size, 0.0], "F")
        .mark([size, 0.0], "F")
        .mark([-2.0 * size, 0.0], "2F")
        .mark([2.0 * size, 0.0], "2F")
        .object([u, 0.0], tip, [0.0, 0.0])
        .ray(tip, [0.0, height])
        .ray(tip, [0.0, 0.0])
        .ray(tip, [0.0, height - slope * u])
        .measured("Image distance, v (cm)", v * PER_M, image_distance(0))
        .measured(
            "Magnification, m = v/u",
            m,
            MeasureSpec::Magnification { object: 0 },
        )
        .answer("Image height (cm)", m * height * PER_M)
        .answer("Real image? (1 = yes)", f64::from(v > 0.0))
        .answer("Power of the lens (D)", 1.0 / f)
        .view(
            [0.5 * (left + right), 0.0],
            0.5 * (right - left),
            1.25 * aperture.max(image_height),
        )
}

fn convex_lens(spec: &ScenarioSpec, p: &Params) -> OpticsScene {
    lens(spec, p, true)
}

fn concave_lens(spec: &ScenarioSpec, p: &Params) -> OpticsScene {
    lens(spec, p, false)
}

static LENS_MAKER: [Param; 4] = [
    param(
        "first",
        "Radius of the first face",
        "cm",
        (10.0, 80.0),
        1.0,
        30.0,
    ),
    param(
        "second",
        "Radius of the second face",
        "cm",
        (10.0, 80.0),
        1.0,
        30.0,
    ),
    param(
        "index",
        "Refractive index of the glass",
        "",
        (1.3, 1.9),
        0.01,
        1.5,
    ),
    param("width", "Width of the beam", "cm", (1.0, 9.0), 0.5, 5.0),
];

/// A ray along the axis from `from`, drawn lightly.
fn hair(from: [f64; 2]) -> SourceSpec {
    SourceSpec::Ray {
        from,
        through: [0.0, from[1]],
        angles: false,
        faint: true,
    }
}

/// How far a spherical cap of this radius stands out at this aperture.
fn sag(radius: f64, aperture: f64) -> f64 {
    radius - (radius * radius - aperture * aperture).sqrt()
}

fn lens_maker(spec: &ScenarioSpec, p: &Params) -> OpticsScene {
    let (first, second) = (p.get(spec, "first") * CM, p.get(spec, "second") * CM);
    let (n, width) = (p.get(spec, "index"), p.get(spec, "width") * CM);
    let aperture = 0.05;
    // Two caps of glass meeting at a sharp edge: a real lens, with thickness, traced surface by surface.
    let thickness = sag(first, aperture) + sag(second, aperture);
    let outline = vec![
        Curve::cap(
            Vec2::new(-0.5 * thickness, 0.0),
            Vec2::new(1.0, 0.0),
            first,
            aperture,
        ),
        Curve::cap(
            Vec2::new(0.5 * thickness, 0.0),
            Vec2::new(-1.0, 0.0),
            second,
            aperture,
        ),
    ];
    // 1/f = (n − 1)(1/R₁ + 1/R₂) for a lens bulging both ways.
    let f = 1.0 / ((n - 1.0) * (1.0 / first + 1.0 / second));
    let start = -0.12;
    let mut scene = OpticsScene::new(spec.title)
        .glass(outline, n)
        .axis([start - 1.0, 0.0], [f + 2.0, 0.0])
        .mark([f, 0.0], "F")
        // Two rays a hair from the axis find the focus the formula is about; the beam shows what the rest do.
        .source(hair([start, 2e-4]))
        .source(hair([start, -2e-4]))
        .source(SourceSpec::Beam {
            centre: [start, 0.0],
            angle: 0.0,
            width,
            count: 9,
        })
        .measured(
            "Focal length, from the lens maker's formula (cm)",
            f * PER_M,
            MeasureSpec::Focus {
                rays: [0, 1],
                origin: [0.0, 0.0],
                axis: [1.0, 0.0],
                scale: PER_M,
            },
        )
        .measured(
            "Where the outermost rays of the beam cross the axis (cm)",
            f * PER_M,
            MeasureSpec::Focus {
                rays: [2, 10],
                origin: [0.0, 0.0],
                axis: [1.0, 0.0],
                scale: PER_M,
            },
        )
        .answer("Power (D)", 1.0 / f)
        .view(
            [0.5 * (start + f * 1.25), 0.0],
            0.5 * (f * 1.25 - start) + 0.03,
            0.085,
        );
    scene.reach = 1.4 * f + 0.2;
    scene
}

static CONTACT: [Param; 2] = [
    param(
        "first",
        "Focal length of the first lens",
        "cm",
        (5.0, 50.0),
        1.0,
        20.0,
    ),
    param(
        "second",
        "Focal length of the second lens",
        "cm",
        (-50.0, 50.0),
        1.0,
        30.0,
    ),
];

fn lenses_in_contact(spec: &ScenarioSpec, p: &Params) -> OpticsScene {
    let first = p.get(spec, "first") * CM;
    // A lens of no power is no lens: keep the second one's focal length off zero.
    let second = match p.get(spec, "second") * CM {
        f if f.abs() < 0.03 => 0.03_f64.copysign(if f == 0.0 { 1.0 } else { f }),
        f => f,
    };
    let power = 1.0 / first + 1.0 / second;
    let f = 1.0 / power;
    // In contact: a thousandth of a millimetre apart, so that each is met in turn.
    let gap = 1e-6;
    let shown = if f.is_finite() && f > 0.0 {
        f.min(1.5)
    } else {
        0.4
    };
    let start = -0.1;
    let mut scene = OpticsScene::new(spec.title)
        .thin_lens(-0.5 * gap, first, 0.05)
        .thin_lens(0.5 * gap, second, 0.05)
        .axis([start - 1.0, 0.0], [shown + 2.0, 0.0])
        .source(SourceSpec::Beam {
            centre: [start, 0.0],
            angle: 0.0,
            width: 0.06,
            count: 7,
        })
        .measured(
            "Focal length of the pair, f₁f₂/(f₁ + f₂) (cm)",
            f * PER_M,
            MeasureSpec::Focus {
                rays: [0, 6],
                origin: [0.0, 0.0],
                axis: [1.0, 0.0],
                scale: PER_M,
            },
        )
        .answer("Power of the first lens (D)", 1.0 / first)
        .answer("Power of the second lens (D)", 1.0 / second)
        .answer("Power of the pair, P₁ + P₂ (D)", power)
        .view(
            [0.5 * (start + shown * 1.2), 0.0],
            0.5 * (shown * 1.2 - start) + 0.03,
            0.08,
        );
    if f.is_finite() && f > 0.0 {
        scene = scene.mark([f, 0.0], "F");
    }
    scene.reach = shown * 1.5 + 0.2;
    scene
}

// ---------------------------------------------------------------- total reflection put to use

static FIBRE: [Param; 3] = [
    param(
        "angle",
        "Angle the light enters at",
        "°",
        (0.0, 80.0),
        1.0,
        20.0,
    ),
    param(
        "core",
        "Refractive index of the core",
        "",
        (1.3, 1.9),
        0.01,
        1.5,
    ),
    param(
        "cladding",
        "Refractive index of the cladding",
        "",
        (1.0, 1.8),
        0.01,
        1.4,
    ),
];

fn optical_fibre(spec: &ScenarioSpec, p: &Params) -> OpticsScene {
    let angle = p.get(spec, "angle").to_radians();
    let core = p.get(spec, "core");
    let cladding = p.get(spec, "cladding").min(core);
    let (length, inner, outer) = (0.5, 0.012, 0.022);
    let inside = (angle.sin() / core).asin();
    let critical = (cladding / core).asin();
    let widest = (core * core - cladding * cladding).sqrt();
    let direction = Vec2::from_polar(1.0, angle);
    let start = -direction * 0.06;
    let mut scene = OpticsScene::new(spec.title)
        .glass(rectangle([0.0, -outer], [length, outer]), cladding)
        .glass(rectangle([0.0, -inner], [length, inner]), core)
        .source(SourceSpec::Ray {
            from: [start.x, start.y],
            through: [0.0, 0.0],
            angles: true,
            faint: false,
        })
        .answer(
            "Critical angle at the wall of the core (°)",
            critical.to_degrees(),
        )
        .measured(
            "Angle it meets the wall at (°)",
            (std::f64::consts::FRAC_PI_2 - inside).to_degrees(),
            MeasureSpec::Incidence { ray: 0, event: 1 },
        )
        .answer(
            "Widest angle the fibre accepts (°)",
            widest.min(1.0).asin().to_degrees(),
        )
        .answer(
            "Carried along the fibre? (1 = yes)",
            f64::from(angle.sin() <= widest),
        )
        .view([0.22, 0.0], 0.32, 0.09);
    scene.reach = 0.2;
    scene
}

// ---------------------------------------------------------------- instruments

static TELESCOPE: [Param; 3] = [
    param(
        "objective",
        "Focal length of the objective",
        "cm",
        (20.0, 80.0),
        1.0,
        40.0,
    ),
    param(
        "eyepiece",
        "Focal length of the eyepiece",
        "cm",
        (2.0, 10.0),
        0.5,
        5.0,
    ),
    param(
        "angle",
        "Angle of the star off the axis",
        "°",
        (0.5, 4.0),
        0.1,
        2.0,
    ),
];

fn telescope(spec: &ScenarioSpec, p: &Params) -> OpticsScene {
    let (objective, eyepiece) = (p.get(spec, "objective") * CM, p.get(spec, "eyepiece") * CM);
    let angle = p.get(spec, "angle").to_radians();
    let length = objective + eyepiece;
    let lens_size = 0.04;
    let reach_of_eyepiece = length * angle.tan() + 0.5 * lens_size * eyepiece / objective + 0.01;
    // Light from a star above the axis comes down across it. The middle ray goes through the centre of the
    // objective.
    let direction = Vec2::from_polar(1.0, -angle);
    let centre = -direction * 0.12;
    let mut scene = OpticsScene::new(spec.title)
        .thin_lens(0.0, objective, lens_size)
        .thin_lens(length, eyepiece, reach_of_eyepiece)
        .axis([-1.0, 0.0], [length + 1.0, 0.0])
        .mark([objective, 0.0], "F")
        .ray([centre.x, centre.y], [0.0, 0.0])
        .source(SourceSpec::Beam {
            centre: [centre.x, centre.y],
            angle: -angle,
            width: 1.4 * lens_size,
            count: 5,
        })
        .measured(
            "Magnifying power, fₒ/fₑ",
            objective / eyepiece,
            MeasureSpec::AngularMagnification {
                ray: 0,
                axis: [1.0, 0.0],
            },
        )
        .answer("Length of the tube, fₒ + fₑ (cm)", length * PER_M)
        .answer(
            "Angle the light leaves the eyepiece at (°)",
            (angle.tan() * objective / eyepiece).atan().to_degrees(),
        )
        .view([0.5 * length - 0.02, 0.0], 0.5 * length + 0.16, 0.1);
    scene.reach = length + 0.2;
    scene
}

static MICROSCOPE: [Param; 3] = [
    param(
        "objective",
        "Focal length of the objective",
        "cm",
        (1.0, 4.0),
        0.1,
        2.0,
    ),
    param(
        "eyepiece",
        "Focal length of the eyepiece",
        "cm",
        (3.0, 10.0),
        0.5,
        5.0,
    ),
    param(
        "beyond",
        "Object beyond the focus by",
        "cm",
        (0.2, 2.0),
        0.1,
        0.5,
    ),
];

/// The least distance of distinct vision.
const NEAR_POINT: f64 = 0.25;

fn compound_microscope(spec: &ScenarioSpec, p: &Params) -> OpticsScene {
    let (objective, eyepiece) = (p.get(spec, "objective") * CM, p.get(spec, "eyepiece") * CM);
    let u = -(objective + p.get(spec, "beyond") * CM);
    let v = u * objective / (u + objective);
    // The eyepiece is placed so that the final image stands at the near point, 25 cm in front of it.
    let before_eyepiece = NEAR_POINT * eyepiece / (NEAR_POINT + eyepiece);
    let length = v + before_eyepiece;
    let by_objective = v / u;
    let by_eyepiece = 1.0 + NEAR_POINT / eyepiece;
    let total = by_objective * by_eyepiece;
    let height = 0.002;
    let tip = [u, height];
    let final_height = (total * height).abs();
    let left = (length - NEAR_POINT).min(u) - 0.02;
    let eyepiece_size = (by_objective * height).abs() * 1.6 + 0.012;
    let mut scene = OpticsScene::new(spec.title)
        .thin_lens(0.0, objective, 0.012)
        .thin_lens(length, eyepiece, eyepiece_size)
        .axis([left - 1.0, 0.0], [length + 1.0, 0.0])
        .mark([objective, 0.0], "Fₒ")
        .mark([length - eyepiece, 0.0], "Fₑ")
        .mark([v, 0.0], "first image")
        .object([u, 0.0], tip, [0.0, 0.0])
        .ray(tip, [0.0, height])
        .ray(tip, [0.0, 0.0])
        .measured(
            "Magnifying power: the objective's times the eyepiece's",
            total,
            MeasureSpec::Magnification { object: 0 },
        )
        .answer("By the objective, v/u", by_objective)
        .answer("By the eyepiece, 1 + D/fₑ", by_eyepiece)
        .answer("Image made by the objective, from it (cm)", v * PER_M)
        .answer("Length of the tube (cm)", length * PER_M)
        .view(
            [0.5 * (left + length + 0.06), -0.3 * final_height],
            0.5 * (length + 0.06 - left),
            0.75 * final_height + 0.02,
        );
    scene.reach = length + 0.1;
    scene
}

// ---------------------------------------------------------------- catalogue

const CHAPTER: &str = "Ray optics and optical instruments";

const fn spec(
    id: &'static str,
    title: &'static str,
    description: &'static str,
    topic: &'static str,
    params: &'static [Param],
) -> ScenarioSpec {
    ScenarioSpec {
        id,
        title,
        description,
        syllabus: Syllabus {
            class: 12,
            chapter: CHAPTER,
            topic,
        },
        params,
    }
}

#[rustfmt::skip]
static CATALOGUE: &[Scenario] = &[
    Scenario {
        spec: spec("concave-mirror", "Image in a concave mirror", "Three rays from the top of the object, each one easy to follow: parallel to the axis, through the centre of curvature, and to the pole. Where they meet again is the image. Bring the object inside the focus and they no longer meet: the image is behind the mirror, upright and enlarged.", "reflection by spherical mirrors", &CONCAVE_MIRROR),
        describe: concave_mirror,
    },
    Scenario {
        spec: spec("convex-mirror", "Image in a convex mirror", "A convex mirror spreads the rays, so they never meet in front of it. Traced back, they seem to come from a small upright image behind the mirror, always between the pole and the focus. That is why it gives a driver a wide view.", "reflection by spherical mirrors", &CONVEX_MIRROR),
        describe: convex_mirror,
    },
    Scenario {
        spec: spec("snells-law", "Refraction at a flat surface", "Light bends where its speed changes: towards the normal going into the slower medium, away from it coming out. Make the medium above the denser one and raise the angle until the light cannot get out at all.", "refraction", &SNELL),
        describe: snells_law,
    },
    Scenario {
        spec: spec("glass-slab", "Through a glass slab", "The slab bends the ray on the way in and bends it back by the same amount on the way out. It leaves in its old direction, moved a little to one side.", "refraction", &SLAB),
        describe: glass_slab,
    },
    Scenario {
        spec: spec("apparent-depth", "Apparent depth", "Rays from a coin under water bend away from the normal as they leave. To an eye above they seem to come from a point higher up, so the water looks shallower than it is.", "refraction", &APPARENT_DEPTH),
        describe: apparent_depth,
    },
    Scenario {
        spec: spec("total-internal-reflection", "Total internal reflection", "A lamp under water. Rays that meet the surface steeply get out, bent away from the normal. Past the critical angle there is no angle left for them to leave at, and the surface turns them all back like a perfect mirror.", "total internal reflection", &TOTAL_REFLECTION),
        describe: total_internal_reflection,
    },
    Scenario {
        spec: spec("optical-fibre", "An optical fibre", "A thread of glass inside a coat of slightly less dense glass. Light sent in nearly along it meets the wall beyond the critical angle every time, and is handed on down the fibre with nothing lost through the side. Send it in too steeply and it leaks out.", "total internal reflection", &FIBRE),
        describe: optical_fibre,
    },
    Scenario {
        spec: spec("spherical-surface", "Refraction at a curved surface", "One curved face of glass is already half a lens. Rays from a point on the axis are bent as they enter and come together inside the glass. Bring the point close enough and they are still spreading after the surface.", "refraction at spherical surfaces", &SURFACE),
        describe: spherical_surface,
    },
    Scenario {
        spec: spec("convex-lens", "Image formed by a convex lens", "Two rays are enough to find an image: the one parallel to the axis leaves through the focus, and the one through the centre goes straight on. Slide the object from far away to inside the focus and watch the image run off to infinity and come back on the other side, upright.", "thin lenses", &CONVEX_LENS),
        describe: convex_lens,
    },
    Scenario {
        spec: spec("concave-lens", "Image formed by a concave lens", "A concave lens spreads the rays. They never meet beyond it; traced back they seem to come from a small upright image on the same side as the object, wherever the object is.", "thin lenses", &CONCAVE_LENS),
        describe: concave_lens,
    },
    Scenario {
        spec: spec("lens-maker", "The lens maker's formula", "A real lens: two curved faces of glass, and Snell's law at each. Curve the faces more, or use denser glass, and the focus moves in. Widen the beam and the outer rays cross short of the focus: a spherical lens does not bring a wide beam to one point.", "the lens maker's formula", &LENS_MAKER),
        describe: lens_maker,
    },
    Scenario {
        spec: spec("lenses-in-contact", "Two lenses in contact", "Put two thin lenses together and their powers add. A concave lens has negative power: make the second one concave and it pushes the focus further off, or cancels the first altogether.", "combination of lenses", &CONTACT),
        describe: lenses_in_contact,
    },
    Scenario {
        spec: spec("prism", "Refraction through a prism", "A prism turns a ray towards its base twice. Change the angle of incidence and the deviation falls, reaches a least value when the ray inside runs parallel to the base, and rises again. That least value is how the refractive index of a glass is measured.", "refraction through a prism", &PRISM),
        describe: prism,
    },
    Scenario {
        spec: spec("telescope", "An astronomical telescope", "Parallel light from a star is brought to a focus by the objective, and the eyepiece, its own focus at the same place, sends it on parallel again but at a much steeper angle. The star looks that many times further from the axis.", "optical instruments", &TELESCOPE),
        describe: telescope,
    },
    Scenario {
        spec: spec("compound-microscope", "A compound microscope", "The objective makes a real, enlarged image of an object just beyond its focus. The eyepiece is a magnifying glass held up to that image. The two enlargements multiply.", "optical instruments", &MICROSCOPE),
        describe: compound_microscope,
    },
];
