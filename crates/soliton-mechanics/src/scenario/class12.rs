//! Class 12: charges and fields (design 011).
//!
//! The charges are those of the laboratory, microcoulombs on beads of a few grams, so that the forces are
//! millinewtons and the motion takes seconds. The laws are the same ones that steer an electron; only the
//! numbers are of a size that can be watched.

use super::{ball, param, round_step, Param, Params, ScenarioSpec, G};
use crate::electromagnetism::COULOMB;
use crate::magnetism::{sphere_polarisability, MAGNETIC};
use crate::scene::{
    AnchorSpec, BodySpec, ConstraintSpec, DistributionSpec, FinishSpec, ForceSpec, RegionSpec,
    SceneFile, ShapeSpec,
};
use std::f64::consts::{PI, TAU};

const MICRO: f64 = 1e-6;
const MILLI: f64 = 1e-3;
const GRAM: f64 = 1e-3;
const MILLIGRAM: f64 = 1e-6;
const BEAD: f64 = 0.05;

/// Seen from above, or with gravity too weak to matter beside the electric force: no gravity in the plane.
fn weightless(spec: &ScenarioSpec, dt: f64, duration: f64) -> SceneFile {
    SceneFile::new(spec.title, [0.0, 0.0], dt).finish(FinishSpec::time(duration))
}

fn fixed_charge(id: &str, charge: f64) -> BodySpec {
    BodySpec::fixed(id, ShapeSpec::Circle(BEAD)).charge(charge)
}

fn bead(id: &str, mass: f64, charge: f64) -> BodySpec {
    ball(id, mass, BEAD).charge(charge)
}

fn coulomb() -> ForceSpec {
    ForceSpec::Electrostatics {
        constant: COULOMB,
        fixed: Vec::new(),
    }
}

/// Coulomb's law with one fixed spread of charge in the scene.
fn coulomb_with(distribution: DistributionSpec) -> ForceSpec {
    ForceSpec::Electrostatics {
        constant: COULOMB,
        fixed: vec![distribution],
    }
}

fn region(min: [f64; 2], max: [f64; 2]) -> Option<RegionSpec> {
    Some(RegionSpec { min, max })
}

/// A thin wall across the path at `x`, placed so that a bead's centre is at `x` when it touches.
fn screen(x: f64, half_height: f64) -> BodySpec {
    BodySpec::fixed("screen", ShapeSpec::Rectangle([0.06, 2.0 * half_height]))
        .at(x + BEAD + 0.03, 0.0)
        .friction(1.0, 1.0)
        .restitution(0.0)
}

// ---------------------------------------------------------------- Electric charges and fields

pub(super) static COULOMB_FORCE: [Param; 4] = [
    param("fixed", "Fixed charge", "μC", (0.5, 10.0), 0.5, 2.0),
    param("free", "Free charge", "μC", (0.5, 10.0), 0.5, 1.0),
    param("separation", "Separation", "m", (0.2, 2.0), 0.05, 0.5),
    param(
        "mass",
        "Mass of the free charge",
        "g",
        (1.0, 50.0),
        1.0,
        10.0,
    ),
];

pub(super) fn coulomb_force(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let (big, small) = (p.get(spec, "fixed") * MICRO, p.get(spec, "free") * MICRO);
    let (r, m) = (p.get(spec, "separation"), p.get(spec, "mass") * GRAM);
    let strength = COULOMB * big * small;
    // The time the pair would take to fly apart if the first instant's push never weakened.
    let scale = (m * r * r * r / strength).sqrt();
    let duration = 2.6 * scale;
    weightless(spec, round_step(duration / 4000.0), duration)
        .playback(duration / 6.0)
        .body(fixed_charge("fixed charge", big))
        .body(bead("free charge", m, small).at(r, 0.0))
        .force(coulomb())
        .focus("free charge")
        .answer("Force at the start (mN)", strength / (r * r) / MILLI)
        .answer("Acceleration at the start (m/s²)", strength / (r * r) / m)
        .answer(
            "Speed at twice the separation (m/s)",
            (strength / (m * r)).sqrt(),
        )
        .answer("Speed far away (m/s)", (2.0 * strength / (m * r)).sqrt())
        .frame([1.6 * r, 0.0], 1.9 * r, 1.1 * r)
}

pub(super) static TWO_CHARGES: [Param; 5] = [
    param("left", "Left charge", "μC", (-10.0, 10.0), 0.5, 4.0),
    param("right", "Right charge", "μC", (-10.0, 10.0), 0.5, -4.0),
    param("separation", "Separation", "m", (0.5, 4.0), 0.1, 2.0),
    param("x", "Test charge, across", "m", (-3.0, 3.0), 0.1, 0.0),
    param("y", "Test charge, up", "m", (-2.0, 2.0), 0.1, 1.0),
];

/// Small enough to feel the field without changing it.
const TEST_CHARGE: f64 = 1e-9;

pub(super) fn field_of_two_charges(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let (left, right) = (p.get(spec, "left") * MICRO, p.get(spec, "right") * MICRO);
    let half = 0.5 * p.get(spec, "separation");
    let (x, y) = (p.get(spec, "x"), p.get(spec, "y"));
    let mut field = [0.0, 0.0];
    let mut potential = 0.0;
    for (at, charge) in [(-half, left), (half, right)] {
        let (dx, dy) = (x - at, y);
        let distance = dx.hypot(dy).max(1e-9);
        field[0] += COULOMB * charge * dx / distance.powi(3);
        field[1] += COULOMB * charge * dy / distance.powi(3);
        potential += COULOMB * charge / distance;
    }
    let size = field[0].hypot(field[1]);
    weightless(spec, spec.dt, spec.duration)
        .body(fixed_charge("left charge", left).at(-half, 0.0))
        .body(fixed_charge("right charge", right).at(half, 0.0))
        .body(ball("test charge", GRAM, 0.03).charge(TEST_CHARGE).at(x, y))
        .force(coulomb())
        .focus("test charge")
        .answer("Field at the test charge (kN/C)", size / 1000.0)
        .answer("Potential at the test charge (kV)", potential / 1000.0)
        .answer("Force on the test charge (μN)", size * TEST_CHARGE / MICRO)
        .frame([0.0, 0.0], half + 2.0, 2.4)
}

pub(super) static CHARGED_PENDULUM: [Param; 4] = [
    param("field", "Electric field", "kN/C", (0.0, 50.0), 1.0, 10.0),
    param("charge", "Charge on the bob", "μC", (0.2, 5.0), 0.1, 1.0),
    param("mass", "Mass of the bob", "g", (1.0, 20.0), 0.5, 2.0),
    param("length", "Length of the thread", "m", (0.3, 2.0), 0.1, 1.0),
];

pub(super) fn charged_pendulum(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let (field, q) = (p.get(spec, "field") * 1000.0, p.get(spec, "charge") * MICRO);
    let (m, length) = (p.get(spec, "mass") * GRAM, p.get(spec, "length"));
    let rest = (q * field).atan2(m * G);
    let pull = (q * field).hypot(m * G);
    SceneFile::new(spec.title, [0.0, -G], spec.dt)
        .finish(FinishSpec::time(spec.duration))
        .body(
            ball("bob", m, 0.06 * length.max(0.5))
                .charge(q)
                .at(0.0, -length),
        )
        .constraint(ConstraintSpec::Rod {
            a: AnchorSpec::Fixed([0.0, 0.0]),
            b: AnchorSpec::body("bob"),
            length: Some(length),
        })
        .force(ForceSpec::ElectricField {
            field: [field, 0.0],
            region: None,
            angular_frequency: 0.0,
            phase: 0.0,
        })
        .focus("bob")
        .answer("Angle it would hang at (°)", rest.to_degrees())
        .answer("Furthest it swings (°)", 2.0 * rest.to_degrees())
        .answer("Electric force (mN)", q * field / MILLI)
        .answer("Tension when hanging at rest (mN)", pull / MILLI)
        .answer(
            "Period of small swings about that angle (s)",
            TAU * (length * m / pull).sqrt(),
        )
        .view([0.3 * length, -0.5 * length], 1.2 * length)
}

pub(super) static PLATES: [Param; 4] = [
    param("speed", "Speed", "m/s", (1.0, 10.0), 0.5, 4.0),
    param(
        "field",
        "Field between the plates",
        "kN/C",
        (0.0, 20.0),
        0.5,
        5.0,
    ),
    param("charge", "Charge", "μC", (-3.0, 3.0), 0.1, 1.0),
    param("length", "Length of the plates", "m", (0.5, 3.0), 0.1, 2.0),
];

const PLATE_GAP: f64 = 0.8;
const TO_SCREEN: f64 = 1.5;
const BEAD_MASS: f64 = 2.0 * GRAM;

pub(super) fn charge_between_plates(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let (v, field) = (p.get(spec, "speed"), p.get(spec, "field") * 1000.0);
    let (q, length) = (p.get(spec, "charge") * MICRO, p.get(spec, "length"));
    let a = q * field / BEAD_MASS;
    let slope = a * length / (v * v);
    let on_screen = slope * (0.5 * length + TO_SCREEN);
    let duration = 1.3 * (length + TO_SCREEN + 0.6) / v;
    weightless(spec, spec.dt, duration)
        .body(
            bead("charge", BEAD_MASS, q)
                .at(-0.6, 0.0)
                .velocity(v, 0.0)
                .friction(1.0, 1.0)
                .restitution(0.0),
        )
        .body(screen(length + TO_SCREEN, on_screen.abs() + 1.2))
        .force(ForceSpec::ElectricField {
            field: [0.0, field],
            region: region([0.0, -PLATE_GAP], [length, PLATE_GAP]),
            angular_frequency: 0.0,
            phase: 0.0,
        })
        .focus("charge")
        .finish(FinishSpec::contact("charge", "screen"))
        .answer("Sideways acceleration between the plates (m/s²)", a)
        .answer("Deflection on leaving the plates (m)", 0.5 * slope * length)
        .answer("Angle on leaving the plates (°)", slope.atan().to_degrees())
        .answer("Where it meets the screen (m)", on_screen)
        .frame(
            [0.5 * (length + TO_SCREEN) - 0.3, 0.5 * on_screen],
            0.5 * (length + TO_SCREEN) + 0.8,
            0.5 * on_screen.abs() + 1.1,
        )
}

pub(super) static DROP: [Param; 3] = [
    param("field", "Field, upward", "kN/C", (0.0, 30.0), 0.2, 12.0),
    param("charge", "Charge on the drop", "μC", (0.2, 5.0), 0.1, 1.0),
    param("mass", "Mass of the drop", "g", (0.2, 5.0), 0.1, 1.0),
];

pub(super) fn balanced_drop(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let (field, q, m) = (
        p.get(spec, "field") * 1000.0,
        p.get(spec, "charge") * MICRO,
        p.get(spec, "mass") * GRAM,
    );
    SceneFile::new(spec.title, [0.0, -G], spec.dt)
        .finish(FinishSpec::time(spec.duration))
        .body(bead("drop", m, q))
        .force(ForceSpec::ElectricField {
            field: [0.0, field],
            region: None,
            angular_frequency: 0.0,
            phase: 0.0,
        })
        .focus("drop")
        .answer("Field that holds it still (kN/C)", m * G / q / 1000.0)
        .answer("Weight (mN)", m * G / MILLI)
        .answer("Electric force (mN)", q * field / MILLI)
        .answer("Acceleration, upward (m/s²)", q * field / m - G)
        .view([0.0, 0.0], 2.5)
}

pub(super) static DIPOLE: [Param; 5] = [
    param("charge", "Each charge", "μC", (0.5, 5.0), 0.1, 2.0),
    param(
        "separation",
        "Distance between them",
        "m",
        (0.2, 1.5),
        0.05,
        0.6,
    ),
    param("field", "Electric field", "kN/C", (1.0, 20.0), 0.5, 5.0),
    param(
        "angle",
        "Angle to the field at release",
        "°",
        (5.0, 175.0),
        1.0,
        30.0,
    ),
    param("mass", "Mass of each end", "g", (1.0, 20.0), 0.5, 5.0),
];

pub(super) fn dipole_in_field(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let (q, d) = (p.get(spec, "charge") * MICRO, p.get(spec, "separation"));
    let (field, angle) = (
        p.get(spec, "field") * 1000.0,
        p.get(spec, "angle").to_radians(),
    );
    let m = p.get(spec, "mass") * GRAM;
    let moment = q * d;
    let inertia = 2.0 * m * (0.5 * d) * (0.5 * d);
    let period = TAU * (inertia / (moment * field)).sqrt();
    let (x, y) = (0.5 * d * angle.cos(), 0.5 * d * angle.sin());
    let duration = 2.2 * period;
    weightless(spec, round_step(period / 3000.0).min(1e-3), duration)
        .body(bead("positive end", m, q).at(x, y))
        .body(bead("negative end", m, -q).at(-x, -y))
        .constraint(ConstraintSpec::Rod {
            a: AnchorSpec::body("positive end"),
            b: AnchorSpec::body("negative end"),
            length: Some(d),
        })
        .force(ForceSpec::ElectricField {
            field: [field, 0.0],
            region: None,
            angular_frequency: 0.0,
            phase: 0.0,
        })
        .focus("positive end")
        .answer("Dipole moment (μC·m)", moment / MICRO)
        .answer(
            "Torque at release (mN·m)",
            moment * field * angle.sin() / MILLI,
        )
        .answer("Force on each end (mN)", q * field / MILLI)
        .answer("Period of small swings (s)", period)
        .view([0.0, 0.0], 0.9 * d + 0.3)
}

pub(super) static GAUSS: [Param; 5] = [
    param("left", "Left charge", "μC", (-10.0, 10.0), 0.5, 4.0),
    param("right", "Right charge", "μC", (-10.0, 10.0), 0.5, -2.0),
    param("x", "Surface, across", "m", (-3.0, 3.0), 0.1, -1.0),
    param("y", "Surface, up", "m", (-2.0, 2.0), 0.1, 0.0),
    param("radius", "Radius of the surface", "m", (0.2, 3.0), 0.1, 0.7),
];

/// `ε₀`, from the constant in Coulomb's law.
const EPSILON: f64 = 1.0 / (4.0 * PI * COULOMB);

pub(super) fn gauss_law(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let (left, right) = (p.get(spec, "left") * MICRO, p.get(spec, "right") * MICRO);
    let (x, y, radius) = (p.get(spec, "x"), p.get(spec, "y"), p.get(spec, "radius"));
    let inside: f64 = [(-1.0, left), (1.0, right)]
        .into_iter()
        .filter(|&(at, _)| (x - at).hypot(y) < radius)
        .map(|(_, charge)| charge)
        .sum();
    weightless(spec, spec.dt, spec.duration)
        .body(fixed_charge("left charge", left).at(-1.0, 0.0))
        .body(fixed_charge("right charge", right).at(1.0, 0.0))
        .force(coulomb())
        .surface([x, y], radius)
        .answer("Charge inside the surface (μC)", inside / MICRO)
        .answer(
            "Flux out of the surface (kN·m²/C)",
            inside / EPSILON / 1000.0,
        )
        .frame([0.0, 0.0], 3.4, 2.5)
}

pub(super) static WIRE: [Param; 4] = [
    param(
        "density",
        "Charge on the wire",
        "μC/m",
        (0.2, 5.0),
        0.1,
        1.0,
    ),
    param(
        "charge",
        "Charge on the bead",
        "μC",
        (-3.0, -0.1),
        0.1,
        -1.0,
    ),
    param("mass", "Mass of the bead", "g", (1.0, 20.0), 0.5, 5.0),
    param(
        "radius",
        "Distance from the wire",
        "m",
        (0.3, 1.5),
        0.05,
        0.8,
    ),
];

pub(super) fn charged_wire(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let (density, q) = (
        p.get(spec, "density") * MICRO,
        p.get(spec, "charge") * MICRO,
    );
    let (m, r) = (p.get(spec, "mass") * GRAM, p.get(spec, "radius"));
    let field = 2.0 * COULOMB * density / r;
    // m v²/r = |q| λ / 2πε₀r: the distance cancels.
    let speed = (2.0 * COULOMB * density * q.abs() / m).sqrt();
    let period = TAU * r / speed;
    let duration = 2.2 * period;
    weightless(spec, round_step(period / 3000.0).min(1e-3), duration)
        .body(bead("bead", m, q).at(r, 0.0).velocity(0.0, speed))
        .force(coulomb_with(DistributionSpec::Wire {
            at: [0.0, 0.0],
            density,
        }))
        .focus("bead")
        .answer("Field at the bead (kN/C)", field / 1000.0)
        .answer("Force on the bead (mN)", q.abs() * field / MILLI)
        .answer("Speed that keeps a circle, at any distance (m/s)", speed)
        .answer("Time for one turn (s)", period)
        .view([0.0, 0.0], 1.25 * r + 0.2)
}

pub(super) static SHEET: [Param; 4] = [
    param(
        "density",
        "Charge on the sheet",
        "μC/m²",
        (0.1, 2.0),
        0.1,
        0.5,
    ),
    param("charge", "Charge on the bead", "μC", (-3.0, 3.0), 0.1, 1.0),
    param("mass", "Mass of the bead", "g", (1.0, 20.0), 0.5, 5.0),
    param(
        "distance",
        "Distance from the sheet",
        "m",
        (0.2, 2.0),
        0.1,
        0.5,
    ),
];

pub(super) fn charged_sheet(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let (density, q) = (
        p.get(spec, "density") * MICRO,
        p.get(spec, "charge") * MICRO,
    );
    let (m, d) = (p.get(spec, "mass") * GRAM, p.get(spec, "distance"));
    let field = density / (2.0 * EPSILON);
    let a = q.abs() * field / m;
    // An opposite charge is drawn in, through a small hole in the sheet, and out the other side as far again.
    let across = 4.0 * (2.0 * d / a.max(1e-9)).sqrt();
    let duration = if q < 0.0 {
        2.2 * across
    } else if q > 0.0 {
        (2.0 * 3.0 / a).sqrt()
    } else {
        2.0
    };
    let mut scene = weightless(spec, round_step(duration / 6000.0).min(1e-3), duration)
        .body(bead("bead", m, q).at(d, 0.0))
        .force(coulomb_with(DistributionSpec::Sheet {
            point: [0.0, 0.0],
            normal: [1.0, 0.0],
            density,
        }))
        .focus("bead")
        .answer("Field of the sheet, at any distance (kN/C)", field / 1000.0)
        .answer("Force on the bead (mN)", q.abs() * field / MILLI)
        .answer("Acceleration (m/s²)", a);
    if q < 0.0 {
        scene = scene.answer("Time to go through and come back (s)", across);
    }
    let half_width = if q < 0.0 { 1.6 * d + 0.4 } else { 2.2 };
    let centre = if q < 0.0 { 0.0 } else { 1.6 };
    scene.frame([centre, 0.0], half_width, 0.65 * half_width)
}

pub(super) static SHELL: [Param; 4] = [
    param("shell", "Charge on the shell", "μC", (0.5, 10.0), 0.5, 4.0),
    param("radius", "Radius of the shell", "m", (0.4, 1.5), 0.1, 0.8),
    param("charge", "Charge on the bead", "μC", (0.1, 3.0), 0.1, 1.0),
    param("speed", "Speed of the bead", "m/s", (0.5, 5.0), 0.1, 2.0),
];

const SHELL_BEAD_MASS: f64 = 5.0 * GRAM;

pub(super) fn charged_shell(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let (big, radius) = (p.get(spec, "shell") * MICRO, p.get(spec, "radius"));
    let (q, v) = (p.get(spec, "charge") * MICRO, p.get(spec, "speed"));
    let (x, y) = (-0.5 * radius, 0.3 * radius);
    // It crosses the shell, through a small hole, where its straight path meets the sphere.
    let inside = ((radius * radius - y * y).sqrt() - x) / v;
    let energy = COULOMB * big * q / radius;
    let far = (v * v + 2.0 * energy / SHELL_BEAD_MASS).sqrt();
    let duration = inside + 3.0 * radius / far;
    weightless(spec, round_step(duration / 6000.0).min(1e-3), duration)
        .body(bead("bead", SHELL_BEAD_MASS, q).at(x, y).velocity(v, 0.0))
        .force(coulomb_with(DistributionSpec::Shell {
            centre: [0.0, 0.0],
            radius,
            charge: big,
        }))
        .focus("bead")
        .answer(
            "Field just outside the shell (kN/C)",
            COULOMB * big / (radius * radius) / 1000.0,
        )
        .answer(
            "Potential anywhere inside (kV)",
            COULOMB * big / radius / 1000.0,
        )
        .answer("Time it spends inside, at a steady speed (s)", inside)
        .answer("Speed far away (m/s)", far)
        .frame([0.8 * radius, 0.3 * radius], 2.6 * radius, 1.7 * radius)
}

// ---------------------------------------------------------------- Electrostatic potential and capacitance

pub(super) static APPROACH: [Param; 4] = [
    param(
        "speed",
        "Speed it is fired at",
        "m/s",
        (1.0, 10.0),
        0.5,
        4.0,
    ),
    param("fixed", "Fixed charge", "μC", (1.0, 20.0), 0.5, 5.0),
    param("charge", "Charge fired at it", "μC", (0.5, 5.0), 0.1, 1.0),
    param("mass", "Its mass", "g", (1.0, 20.0), 0.5, 5.0),
];

const FIRED_FROM: f64 = 3.0;

pub(super) fn closest_approach(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let (v, big) = (p.get(spec, "speed"), p.get(spec, "fixed") * MICRO);
    let (q, m) = (p.get(spec, "charge") * MICRO, p.get(spec, "mass") * GRAM);
    let strength = COULOMB * big * q;
    let kinetic = 0.5 * m * v * v;
    let total = kinetic + strength / FIRED_FROM;
    let duration = 2.6 * FIRED_FROM / v;
    weightless(spec, round_step(duration / 6000.0).min(1e-3), duration)
        .body(fixed_charge("fixed charge", big))
        .body(bead("charge", m, q).at(FIRED_FROM, 0.0).velocity(-v, 0.0))
        .force(coulomb())
        .focus("charge")
        .answer("Closest distance (m)", strength / total)
        .answer("Kinetic energy at the start (mJ)", kinetic / MILLI)
        .answer(
            "Potential energy at the start (mJ)",
            strength / FIRED_FROM / MILLI,
        )
        .answer("Potential energy at the closest point (mJ)", total / MILLI)
        .frame([0.5 * FIRED_FROM, 0.0], 0.5 * FIRED_FROM + 0.6, 1.0)
}

pub(super) static VOLTAGE: [Param; 3] = [
    param(
        "voltage",
        "Potential difference",
        "kV",
        (0.5, 20.0),
        0.5,
        5.0,
    ),
    param("mass", "Mass", "g", (0.5, 5.0), 0.1, 1.0),
    param(
        "gap",
        "Distance between the plates",
        "m",
        (0.5, 2.0),
        0.1,
        1.0,
    ),
];

pub(super) fn accelerating_voltage(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let (voltage, m, gap) = (
        p.get(spec, "voltage") * 1000.0,
        p.get(spec, "mass") * GRAM,
        p.get(spec, "gap"),
    );
    let q = MICRO;
    let speed = (2.0 * q * voltage / m).sqrt();
    let crossing = 2.0 * gap / speed;
    weightless(spec, spec.dt, crossing + 1.0 / speed)
        .body(bead("charge", m, q))
        .force(ForceSpec::ElectricField {
            field: [voltage / gap, 0.0],
            region: region([0.0, -PLATE_GAP], [gap, PLATE_GAP]),
            angular_frequency: 0.0,
            phase: 0.0,
        })
        .focus("charge")
        .answer("Speed on leaving (m/s)", speed)
        .answer("Kinetic energy gained, qV (mJ)", q * voltage / MILLI)
        .answer("Field between the plates (kN/C)", voltage / gap / 1000.0)
        .answer("Time between the plates (s)", crossing)
        .frame([0.5 * gap + 0.4, 0.0], 0.5 * gap + 1.0, 1.1)
}

// ---------------------------------------------------------------- Moving charges and magnetism

pub(super) static CIRCLE: [Param; 4] = [
    param("speed", "Speed", "m/s", (0.5, 8.0), 0.1, 3.0),
    param(
        "field",
        "Magnetic field, out of the page",
        "T",
        (0.2, 3.0),
        0.1,
        1.0,
    ),
    param("charge", "Charge", "μC", (0.2, 3.0), 0.1, 1.0),
    param("mass", "Mass", "mg", (0.1, 2.0), 0.1, 0.5),
];

pub(super) fn charge_in_magnetic_field(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let (v, b) = (p.get(spec, "speed"), p.get(spec, "field"));
    let (q, m) = (
        p.get(spec, "charge") * MICRO,
        p.get(spec, "mass") * MILLIGRAM,
    );
    let radius = m * v / (q * b);
    let period = TAU * m / (q * b);
    let duration = 2.05 * period;
    // With the field out of the page a positive charge turns clockwise: from the top of the circle it heads right.
    weightless(spec, round_step(period / 4000.0).min(1e-3), duration)
        .body(
            ball("charge", m, 0.05 * radius.max(0.4))
                .charge(q)
                .at(0.0, radius)
                .velocity(v, 0.0),
        )
        .force(ForceSpec::MagneticField {
            strength: b,
            region: None,
        })
        .focus("charge")
        .answer("Radius of the circle (m)", radius)
        .answer("Time for one turn (s)", period)
        .answer("Turns each second (Hz)", 1.0 / period)
        .answer("Magnetic force (μN)", q * v * b / MICRO)
        .view([0.0, 0.0], 1.3 * radius)
}

pub(super) static SELECTOR: [Param; 3] = [
    param("speed", "Speed", "m/s", (1.0, 8.0), 0.1, 4.0),
    param(
        "electric",
        "Electric field, upward",
        "N/C",
        (1.0, 10.0),
        0.5,
        4.0,
    ),
    param(
        "magnetic",
        "Magnetic field, out of the page",
        "T",
        (0.5, 3.0),
        0.1,
        1.0,
    ),
];

const SELECTOR_LENGTH: f64 = 3.0;

pub(super) fn velocity_selector(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let (v, electric, magnetic) = (
        p.get(spec, "speed"),
        p.get(spec, "electric"),
        p.get(spec, "magnetic"),
    );
    let (q, m) = (MICRO, MILLIGRAM);
    let crossed = region([0.0, -PLATE_GAP], [SELECTOR_LENGTH, PLATE_GAP]);
    let duration = 1.5 * (SELECTOR_LENGTH + 1.6) / v;
    weightless(spec, spec.dt, duration)
        .body(
            bead("charge", m, q)
                .at(-0.6, 0.0)
                .velocity(v, 0.0)
                .friction(1.0, 1.0)
                .restitution(0.0),
        )
        .body(screen(SELECTOR_LENGTH + 1.0, 1.6))
        .force(ForceSpec::ElectricField {
            field: [0.0, electric],
            region: crossed,
            angular_frequency: 0.0,
            phase: 0.0,
        })
        .force(ForceSpec::MagneticField {
            strength: magnetic,
            region: crossed,
        })
        .focus("charge")
        .finish(FinishSpec::contact("charge", "screen"))
        .answer(
            "Speed that goes straight through, E/B (m/s)",
            electric / magnetic,
        )
        .answer("Electric force, upward (μN)", q * electric / MICRO)
        .answer(
            "Magnetic force at this speed, downward (μN)",
            q * v * magnetic / MICRO,
        )
        .frame(
            [0.5 * SELECTOR_LENGTH + 0.2, 0.0],
            0.5 * SELECTOR_LENGTH + 1.1,
            1.5,
        )
}

pub(super) static CYCLOTRON: [Param; 3] = [
    param("field", "Magnetic field", "T", (0.5, 2.0), 0.1, 1.0),
    param(
        "voltage",
        "Voltage across the gap",
        "V",
        (0.05, 0.5),
        0.01,
        0.2,
    ),
    param("radius", "Radius of the magnet", "m", (1.0, 3.0), 0.1, 2.0),
];

const DEE_GAP: f64 = 0.2;

pub(super) fn cyclotron(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let (b, voltage, radius) = (
        p.get(spec, "field"),
        p.get(spec, "voltage"),
        p.get(spec, "radius"),
    );
    let (q, m) = (MICRO, MILLIGRAM);
    let omega = q * b / m;
    let top_speed = q * b * radius / m;
    let top_energy = 0.5 * m * top_speed * top_speed;
    let crossings = (top_energy / (q * voltage)).ceil();
    let duration = (crossings + 4.0) * 0.5 * TAU / omega;
    weightless(spec, round_step(TAU / omega / 4000.0).min(1e-3), duration)
        .playback(duration / 30.0)
        .body(ball("charge", m, 0.04).charge(q))
        .force(ForceSpec::MagneticField {
            strength: b,
            region: region([-radius, -radius], [radius, radius]),
        })
        .force(ForceSpec::ElectricField {
            field: [voltage / DEE_GAP, 0.0],
            region: region([-0.5 * DEE_GAP, -radius], [0.5 * DEE_GAP, radius]),
            angular_frequency: omega,
            phase: 0.0,
        })
        .focus("charge")
        .answer("Time for one turn, at any speed (s)", TAU / omega)
        .answer("Frequency the voltage must alternate at (Hz)", omega / TAU)
        .answer(
            "Energy gained at each crossing, qV (μJ)",
            q * voltage / MICRO,
        )
        .answer("Most speed the magnet can hold, qBR/m (m/s)", top_speed)
        .answer("Most kinetic energy (μJ)", top_energy / MICRO)
        .view([0.0, 0.0], 1.25 * radius)
}

// ---------------------------------------------------------------- Magnetism and matter

const CENTI: f64 = 1e-2;
const NEEDLE_MOMENT: f64 = 1e-3;
const NEEDLE_LENGTH: f64 = 0.05;

fn magnets(background: [f64; 2]) -> ForceSpec {
    ForceSpec::Magnetostatics {
        constant: MAGNETIC,
        background,
    }
}

/// A fixed bar magnet at the origin with its poles at its ends, north towards `angle`.
fn bar_magnet(moment: f64, length: f64, angle: f64) -> BodySpec {
    BodySpec::fixed("magnet", ShapeSpec::Rectangle([length, 0.3 * length]))
        .angle(angle)
        .magnet(moment, length)
}

/// The field at `(x, y)` of a magnet at the origin with its poles `length` apart, north towards `angle`.
fn magnet_field(moment: f64, length: f64, angle: f64, x: f64, y: f64) -> [f64; 2] {
    let pole = moment / length;
    let (ux, uy) = (0.5 * length * angle.cos(), 0.5 * length * angle.sin());
    let mut field = [0.0, 0.0];
    for (px, py, strength) in [(ux, uy, pole), (-ux, -uy, -pole)] {
        let (dx, dy) = (x - px, y - py);
        let distance = dx.hypot(dy).max(1e-9);
        field[0] += MAGNETIC * strength * dx / distance.powi(3);
        field[1] += MAGNETIC * strength * dy / distance.powi(3);
    }
    field
}

/// A small compass on a pivot at `(x, y)`, already pointing the way `field` turns it so that it sits still.
/// Its two ends are in slightly different fields, so that is along the sum of the two, not the field at its
/// middle.
fn compass(scene: SceneFile, x: f64, y: f64, field: impl Fn(f64, f64) -> [f64; 2]) -> SceneFile {
    let half = 0.5 * NEEDLE_LENGTH;
    let middle = field(x, y);
    let mut angle = middle[1].atan2(middle[0]);
    for _ in 0..30 {
        let (ux, uy) = (half * angle.cos(), half * angle.sin());
        let (north, south) = (field(x + ux, y + uy), field(x - ux, y - uy));
        angle = (north[1] + south[1]).atan2(north[0] + south[0]);
    }
    let needle = ShapeSpec::Rectangle([NEEDLE_LENGTH, 0.2 * NEEDLE_LENGTH]);
    let mut scene = scene
        .body(
            BodySpec::rigid("compass", GRAM, needle)
                .at(x, y)
                .angle(angle)
                .magnet(NEEDLE_MOMENT, NEEDLE_LENGTH),
        )
        .constraint(ConstraintSpec::Pin {
            a: AnchorSpec::body("compass"),
            b: AnchorSpec::Fixed([x, y]),
        })
        // The picture is of the field, not of the forces that hold the compass on its pivot.
        .focus("magnet");
    scene
        .ignore_collisions
        .push(["magnet".to_string(), "compass".to_string()]);
    scene
}

pub(super) static BAR_MAGNET: [Param; 4] = [
    param(
        "moment",
        "Moment of the magnet",
        "A·m²",
        (0.2, 5.0),
        0.1,
        1.0,
    ),
    param(
        "length",
        "Length of the magnet",
        "cm",
        (4.0, 16.0),
        1.0,
        8.0,
    ),
    param("x", "Compass, across", "cm", (-28.0, 28.0), 1.0, 18.0),
    param("y", "Compass, up", "cm", (-20.0, 20.0), 1.0, 0.0),
];

pub(super) fn bar_magnet_field(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let (moment, length) = (p.get(spec, "moment"), p.get(spec, "length") * CENTI);
    let (x, y) = (p.get(spec, "x") * CENTI, p.get(spec, "y") * CENTI);
    let field = magnet_field(moment, length, 0.0, x, y);
    let r = x.hypot(y).max(1e-6);
    let far = MAGNETIC * moment / r.powi(3);
    let scene = weightless(spec, spec.dt, spec.duration)
        .body(bar_magnet(moment, length, 0.0))
        .force(magnets([0.0, 0.0]));
    compass(scene, x, y, |x, y| magnet_field(moment, length, 0.0, x, y))
        .answer(
            "Field at the compass (μT)",
            field[0].hypot(field[1]) / MICRO,
        )
        .answer(
            "On the axis at that distance, far away: μ₀2m/4πr³ (μT)",
            2.0 * far / MICRO,
        )
        .answer(
            "Across the middle at that distance, far away: μ₀m/4πr³ (μT)",
            far / MICRO,
        )
        .frame([0.0, 0.0], 0.3, 0.21)
}

pub(super) static NEEDLE: [Param; 5] = [
    param("field", "Magnetic field", "μT", (10.0, 100.0), 5.0, 40.0),
    param(
        "moment",
        "Moment of the needle",
        "mA·m²",
        (5.0, 50.0),
        1.0,
        20.0,
    ),
    param("mass", "Mass of the needle", "g", (0.5, 5.0), 0.1, 1.0),
    param("length", "Length of the needle", "cm", (2.0, 8.0), 0.5, 4.0),
    param(
        "angle",
        "Angle it is let go at",
        "°",
        (5.0, 90.0),
        5.0,
        20.0,
    ),
];

pub(super) fn compass_needle(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let (field, moment) = (p.get(spec, "field") * MICRO, p.get(spec, "moment") * MILLI);
    let (m, length) = (p.get(spec, "mass") * GRAM, p.get(spec, "length") * CENTI);
    let angle = p.get(spec, "angle").to_radians();
    let width = 0.12 * length;
    let inertia = m * (length * length + width * width) / 12.0;
    let period = TAU * (inertia / (moment * field)).sqrt();
    let duration = 3.2 * period;
    // The field points up the page, north; the needle is let go turned away from it.
    weightless(spec, round_step(period / 3000.0).min(1e-3), duration)
        .body(
            BodySpec::rigid("needle", m, ShapeSpec::Rectangle([length, width]))
                .angle(0.5 * PI - angle)
                .magnet(moment, length),
        )
        .constraint(ConstraintSpec::Pin {
            a: AnchorSpec::body("needle"),
            b: AnchorSpec::Fixed([0.0, 0.0]),
        })
        .force(magnets([0.0, field]))
        .focus("needle")
        .answer(
            "Torque when let go, mB sin θ (μN·m)",
            moment * field * angle.sin() / MICRO,
        )
        .answer("Period of small swings of the needle (s)", period)
        .answer(
            "Fastest it turns (rad/s)",
            (2.0 * moment * field * (1.0 - angle.cos()) / inertia).sqrt(),
        )
        .frame([0.0, 0.0], 1.3 * length, 0.9 * length)
}

pub(super) static NEUTRAL: [Param; 6] = [
    param(
        "moment",
        "Moment of the magnet",
        "A·m²",
        (0.2, 5.0),
        0.1,
        1.0,
    ),
    param(
        "length",
        "Length of the magnet",
        "cm",
        (4.0, 16.0),
        1.0,
        8.0,
    ),
    param("earth", "Earth's field", "μT", (10.0, 60.0), 1.0, 35.0),
    param(
        "turn",
        "North pole turned from north by",
        "°",
        (0.0, 180.0),
        180.0,
        0.0,
    ),
    param("x", "Compass, across", "cm", (-28.0, 28.0), 1.0, 10.0),
    param("y", "Compass, up", "cm", (-20.0, 20.0), 1.0, 10.0),
];

pub(super) fn neutral_point(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let (moment, length) = (p.get(spec, "moment"), p.get(spec, "length") * CENTI);
    let earth = p.get(spec, "earth") * MICRO;
    let reversed = p.get(spec, "turn") >= 90.0;
    let (x, y) = (p.get(spec, "x") * CENTI, p.get(spec, "y") * CENTI);
    let angle = if reversed { -0.5 * PI } else { 0.5 * PI };
    let half = 0.5 * length;
    let distance = if reversed {
        // On the axis the magnet's field, μ₀2md/4π(d² − l²)², runs along its moment, here against the earth's.
        let on_axis = |d: f64| MAGNETIC * 2.0 * moment * d / (d * d - half * half).powi(2);
        let (mut near, mut far) = (half * 1.0001, 10.0);
        for _ in 0..80 {
            let middle = 0.5 * (near + far);
            if on_axis(middle) > earth {
                near = middle;
            } else {
                far = middle;
            }
        }
        0.5 * (near + far)
    } else {
        // Beside the magnet its field, μ₀m/4π(d² + l²)^{3/2}, runs against its moment and so against the earth's.
        ((MAGNETIC * moment / earth).powf(2.0 / 3.0) - half * half)
            .max(0.0)
            .sqrt()
    };
    let both = |x: f64, y: f64| {
        let own = magnet_field(moment, length, angle, x, y);
        [own[0], own[1] + earth]
    };
    let total = both(x, y);
    let scene = weightless(spec, spec.dt, spec.duration)
        .body(bar_magnet(moment, length, angle))
        .force(magnets([0.0, earth]));
    compass(scene, x, y, both)
        .answer(
            "Field at the compass (μT)",
            total[0].hypot(total[1]) / MICRO,
        )
        .answer(
            if reversed {
                "Neutral points: north and south of the centre by (cm)"
            } else {
                "Neutral points: east and west of the centre by (cm)"
            },
            distance / CENTI,
        )
        .frame([0.0, 0.0], 0.3, 0.21)
}

pub(super) static MATERIALS: [Param; 3] = [
    param(
        "susceptibility",
        "Susceptibility χ of the sample",
        "×10⁻⁵",
        (-20.0, 30.0),
        0.2,
        -16.6,
    ),
    param(
        "moment",
        "Moment of the magnet",
        "A·m²",
        (5.0, 30.0),
        1.0,
        16.0,
    ),
    param(
        "gap",
        "Gap between pole and sample",
        "cm",
        (2.0, 6.0),
        0.5,
        3.0,
    ),
];

const MAGNET_SIZE: f64 = 0.025;
const SAMPLE_VOLUME: f64 = 1e-6;
const SAMPLE_MASS: f64 = 10.0 * GRAM;

pub(super) fn magnetic_materials(spec: &ScenarioSpec, p: &Params) -> SceneFile {
    let chi = p.get(spec, "susceptibility") * 1e-5;
    let (moment, gap) = (p.get(spec, "moment"), p.get(spec, "gap") * CENTI);
    let half = 0.5 * MAGNET_SIZE;
    let x = half + gap;
    let pole = MAGNETIC * moment / MAGNET_SIZE;
    let (near, far) = (x - half, x + half);
    let field = pole * (near.powi(-2) - far.powi(-2));
    let slope = -2.0 * pole * (near.powi(-3) - far.powi(-3));
    // F = (m·∇)B with m = αB: towards the stronger field for a paramagnet, away from it for a diamagnet.
    let force = sphere_polarisability(chi, SAMPLE_VOLUME) * field * slope;
    let a = force.abs() / SAMPLE_MASS;
    let duration = if a > 0.0 {
        3.0 * (2.0 * gap / a).sqrt()
    } else {
        10.0
    };
    let radius = (3.0 * SAMPLE_VOLUME / (4.0 * PI)).cbrt();
    weightless(spec, round_step(duration / 6000.0).min(1e-3), duration)
        .playback(duration / 10.0)
        .body(
            BodySpec::fixed("magnet", ShapeSpec::Rectangle([MAGNET_SIZE, MAGNET_SIZE]))
                .magnet(moment, MAGNET_SIZE)
                .friction(1.0, 1.0)
                .restitution(0.0),
        )
        .body(
            ball("sample", SAMPLE_MASS, radius)
                .at(x, 0.0)
                .magnetisable(chi, SAMPLE_VOLUME)
                .friction(1.0, 1.0)
                .restitution(0.0),
        )
        .force(magnets([0.0, 0.0]))
        .focus("sample")
        .answer("Field at the sample (mT)", field / MILLI)
        .answer("Force at the start (μN)", force.abs() / MICRO)
        .answer("Acceleration at the start (mm/s²)", a / MILLI)
        .frame([0.05, 0.0], 0.11, 0.07)
}
