//! Bar magnets and magnetic materials (design 012): each scenario against its textbook answer, and the
//! conservation laws that should hold while it runs.

use soliton_mechanics::scenario::{find, Params};
use soliton_mechanics::scene::SceneFile;
use soliton_mechanics::{
    sphere_polarisability, Body, BodyId, Magnet, Magnetostatics, Polygon, Recorder, Simulate,
    Simulation, Vec2, World, MAGNETIC,
};

const STEP_LIMIT: usize = 400_000;

/// The compass of the field scenarios is their second body; the magnet is the one in focus.
const COMPASS: BodyId = BodyId::from_index(1);

struct Run {
    sim: Simulation,
    recorder: Recorder,
    focus: BodyId,
    answers: Vec<(String, f64)>,
}

impl Run {
    fn answer(&self, name: &str) -> f64 {
        self.answers
            .iter()
            .find(|(n, _)| n.starts_with(name))
            .map(|(_, v)| *v)
            .unwrap_or_else(|| panic!("no answer `{name}`"))
    }
}

fn run(id: &str, params: Params, seconds: f64) -> Run {
    let scene = find(id).unwrap().build(&params);
    let (focus, answers) = (scene.focus, scene.exact.clone());
    let mut sim = Simulation::new(scene.world, scene.dt);
    sim.finish_when(scene.finish);
    let mut recorder = Recorder::every(1);
    recorder.observe(&sim);
    let mut steps = 0;
    while sim.finished().is_none() && sim.time() < seconds - 0.5 * sim.dt() {
        sim.step();
        recorder.observe(&sim);
        steps += 1;
        assert!(steps < STEP_LIMIT, "{id}: no end after {STEP_LIMIT} steps");
    }
    Run {
        sim,
        recorder,
        focus,
        answers,
    }
}

fn relative(measured: f64, expected: f64) -> f64 {
    (measured - expected).abs() / expected.abs().max(1e-300)
}

fn bar(moment: f64, length: f64) -> World {
    let mut world = World::new(Vec2::ZERO);
    world.add(Body::fixed(Polygon::rectangle(length, 0.02)).magnet(Magnet::new(moment, length)));
    world.add_force(Magnetostatics::default());
    world
}

#[test]
fn a_bar_magnets_field_is_twice_as_strong_on_its_axis_as_beside_it() {
    let (m, l) = (1.2, 0.04);
    let world = bar(m, 2.0 * l);
    for r in [0.1, 0.25] {
        let axis = world.magnetic_field_in_plane(Vec2::new(r, 0.0));
        let beside = world.magnetic_field_in_plane(Vec2::new(0.0, r));
        // Along the moment on the axis, against it beside the middle: the textbook's two formulas.
        assert!(relative(axis.x, MAGNETIC * 2.0 * m * r / (r * r - l * l).powi(2)) < 1e-12);
        assert!(relative(beside.x, -MAGNETIC * m / (r * r + l * l).powf(1.5)) < 1e-12);
        assert!(axis.y.abs() < 1e-18 && beside.y.abs() < 1e-18);
    }
    // Far away it is a dipole, whatever its length.
    let far = 4.0;
    let ratio = world.magnetic_field_in_plane(Vec2::new(far, 0.0)).x
        / -world.magnetic_field_in_plane(Vec2::new(0.0, far)).x;
    assert!(relative(ratio, 2.0) < 1e-3, "{ratio}");
}

#[test]
fn a_compass_points_along_the_field_and_stays_there() {
    for (x, y) in [(18.0, 0.0), (0.0, 15.0), (12.0, -9.0)] {
        let r = run(
            "bar-magnet-field",
            Params::default().set("x", x).set("y", y),
            1.0,
        );
        let world = r.sim.world();
        let field = world.magnetic_field_on(COMPASS);
        // The needle has some length, so it sits along the field averaged over it: within a few degrees of
        // the field at its middle.
        let needle = Vec2::from_polar(1.0, world.bodies().angle(COMPASS));
        assert!(needle.cross(field).abs() < 0.1 * field.length());
        assert!(needle.dot(field) > 0.0);
        assert!(world.bodies().angular_velocity(COMPASS).abs() < 1e-9);
        assert!(relative(field.length() / 1e-6, r.answer("Field at the compass")) < 1e-9);
    }
    // On the axis, far enough away, the dipole formula is good to a few per cent.
    let r = run("bar-magnet-field", Params::default().set("x", 28.0), 0.1);
    assert!(relative(r.answer("Field at the compass"), r.answer("On the axis")) < 0.05);
}

#[test]
fn a_needle_swings_with_the_period_its_moment_and_inertia_give() {
    let r = run("compass-needle", Params::default().set("angle", 5.0), 60.0);
    let spin = &r.recorder.body(r.focus).angular_velocity;
    let dt = r.sim.dt();
    // Moments the needle turns round the same way.
    let turns: Vec<f64> = (1..spin.len())
        .filter(|&i| spin[i - 1] > 0.0 && spin[i] <= 0.0)
        .map(|i| (i as f64 - 1.0 + spin[i - 1] / (spin[i - 1] - spin[i])) * dt)
        .collect();
    assert!(turns.len() >= 3, "{} swings", turns.len());
    let period = (turns[turns.len() - 1] - turns[0]) / (turns.len() - 1) as f64;
    assert!(
        relative(period, r.answer("Period of small swings")) < 2e-3,
        "{period}"
    );
    let energy = &r.recorder.energy;
    let drift = (energy.last().unwrap().conserved() - energy[0].conserved()).abs();
    assert!(drift < 1e-6 * energy[0].potential.abs(), "drift {drift}");
}

#[test]
fn a_needle_let_go_from_a_large_angle_turns_fastest_through_north() {
    let r = run("compass-needle", Params::default().set("angle", 60.0), 60.0);
    let spin = &r.recorder.body(r.focus).angular_velocity;
    let fastest = spin.iter().fold(0.0f64, |most, w| most.max(w.abs()));
    assert!(
        relative(fastest, r.answer("Fastest it turns")) < 1e-4,
        "{fastest}"
    );
    // No net force in a uniform field: the needle turns where it is.
    let moved = r.recorder.body(r.focus).position.last().unwrap().length();
    assert!(moved < 1e-9, "the needle drifted {moved} m");
}

#[test]
fn the_field_vanishes_at_the_neutral_points() {
    let earth = 35e-6;
    for (turn, name) in [(0.0, "east and west"), (180.0, "north and south")] {
        let scene = find("neutral-point")
            .unwrap()
            .build(&Params::default().set("turn", turn));
        let (label, distance) = scene
            .exact
            .iter()
            .find(|(n, _)| n.starts_with("Neutral points"))
            .unwrap();
        assert!(label.contains(name), "{label}");
        let d = distance / 100.0;
        let at = if turn == 0.0 {
            Vec2::new(d, 0.0)
        } else {
            Vec2::new(0.0, d)
        };
        // Carry the compass there: what it reads leaves out its own field.
        for point in [at, -at] {
            let there = find("neutral-point").unwrap().build(
                &Params::default()
                    .set("turn", turn)
                    .set("x", 100.0 * point.x)
                    .set("y", 100.0 * point.y),
            );
            let field = there.world.magnetic_field_on(COMPASS);
            assert!(field.length() < 1e-9 * earth, "{field:?} at {point:?}");
        }
    }
}

#[test]
fn a_diamagnet_is_pushed_out_and_a_paramagnet_pulled_in() {
    for (chi, away) in [(-16.6, 1.0), (2.2, -1.0)] {
        let r = run(
            "magnetic-materials",
            Params::default().set("susceptibility", chi),
            1e9,
        );
        let record = r.recorder.body(r.focus);
        let force = r.recorder.force_named(r.focus, "magnetic").unwrap()[1];
        assert!(force.x * away > 0.0 && force.y.abs() < 1e-12 * force.x.abs());
        assert!(relative(force.x.abs() / 1e-6, r.answer("Force at the start")) < 1e-6);
        let field = r.sim.world().magnetic_field_on(r.focus);
        assert!(field.length() > 0.0);
        let moved = record.position.last().unwrap().x - record.position[0].x;
        assert!(moved * away > 0.005, "it moved {moved} m");
    }
}

#[test]
fn a_magnetised_sample_keeps_its_energy_until_it_touches() {
    let r = run("magnetic-materials", Params::default(), 1e9);
    let energy = &r.recorder.energy;
    let scale = energy.iter().map(|e| e.kinetic.abs()).fold(0.0, f64::max);
    let drift = (energy.last().unwrap().conserved() - energy[0].conserved()).abs();
    assert!(drift < 1e-6 * scale, "drift {drift} against {scale}");
}

#[test]
fn free_magnets_conserve_momentum_angular_momentum_and_energy() {
    let mut world = World::new(Vec2::ZERO);
    let shape = || Polygon::rectangle(0.06, 0.01);
    let a = world.add(
        Body::rigid(0.02, shape())
            .at(-0.1, 0.0)
            .angle(0.4)
            .velocity(0.0, 0.05)
            .magnet(Magnet::new(0.5, 0.06)),
    );
    let b = world.add(
        Body::rigid(0.05, shape())
            .at(0.1, 0.03)
            .angle(2.0)
            .velocity(0.0, -0.02)
            .magnet(Magnet::new(0.8, 0.06)),
    );
    // A bismuth-like bead near them, pushed by both and pushing back.
    let bead = world.add(
        Body::particle(0.01)
            .at(0.0, 0.12)
            .polarisability(sphere_polarisability(-0.9, 1e-6)),
    );
    world.add_force(Magnetostatics::default());
    let mut sim = Simulation::new(world, 2e-5);
    let (momentum, spin, energy) = (
        sim.momentum(),
        sim.world().angular_momentum(),
        sim.energy().conserved(),
    );
    let start = sim.world().bodies().position(bead);
    sim.advance(5_000);
    let bodies = sim.world().bodies();
    assert!(
        (bodies.position(a) - bodies.position(b)).length() > 0.07,
        "they met"
    );
    let moved = (bodies.position(bead) - start).length();
    assert!(moved > 1e-9, "the bead moved only {moved} m");
    assert!((sim.momentum() - momentum).length() < 1e-15);
    assert!((sim.world().angular_momentum() - spin).abs() < 1e-12);
    assert!(relative(sim.energy().conserved(), energy) < 1e-6);
}

#[test]
fn field_lines_run_from_the_north_pole_round_to_the_south() {
    let world = bar(1.0, 0.08);
    let lines = world.magnetic_field_lines(12, Vec2::ZERO, 0.6);
    let (north, south) = (Vec2::new(0.04, 0.0), Vec2::new(-0.04, 0.0));
    let from_north = |line: &&Vec<Vec2>| (line[0] - north).length() < 1e-9;
    let to_south = |line: &&Vec<Vec2>| (*line.last().unwrap() - south).length() < 1e-9;
    assert_eq!(lines.iter().filter(from_north).count(), 12);
    let landed = lines.iter().filter(from_north).filter(to_south).count();
    assert!(
        landed >= 8,
        "{landed} of 12 lines came round to the south pole"
    );
    // The widest loops leave the picture; their far halves are drawn coming back in to the south pole.
    let returning: Vec<&Vec<Vec2>> = lines.iter().filter(|l| !from_north(l)).collect();
    assert_eq!(returning.len(), 12 - landed);
    assert!(returning.iter().all(to_south));
}

#[test]
fn magnets_and_samples_survive_a_round_trip_through_json() {
    for id in [
        "bar-magnet-field",
        "compass-needle",
        "neutral-point",
        "magnetic-materials",
    ] {
        let file = find(id).unwrap().scene_file(&Params::default());
        let back = SceneFile::from_json(&file.to_json()).unwrap();
        assert_eq!(file, back, "{id}");
        back.build().unwrap();
    }
}
