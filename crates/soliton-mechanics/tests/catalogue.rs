use soliton_mechanics::scenario::{catalogue, find, Params, Scene};
use soliton_mechanics::{BodyId, ForceLabel, Recorder, Simulate, Simulation};

fn build(id: &str, params: Params) -> (Scene, Vec<(String, f64)>) {
    let scene = find(id).unwrap().build(&params);
    let answers = scene.exact.clone();
    (scene, answers)
}

fn exact(answers: &[(String, f64)], name: &str) -> f64 {
    answers
        .iter()
        .find(|(n, _)| n.starts_with(name))
        .map(|(_, v)| *v)
        .unwrap_or_else(|| panic!("no exact `{name}`"))
}

fn relative(measured: f64, expected: f64) -> f64 {
    (measured - expected).abs() / expected.abs().max(1e-300)
}

#[test]
fn every_scenario_builds_and_runs_with_defaults() {
    for scenario in catalogue() {
        let scene = scenario.build(&Params::default());
        assert_eq!(
            scene.labels.len(),
            scene.world.bodies().len(),
            "{}: one label per body",
            scenario.spec.id
        );
        assert!(
            !scene.exact.is_empty(),
            "{}: exact answers",
            scenario.spec.id
        );
        let mut sim = Simulation::new(scene.world, scene.dt);
        let mut recorder = Recorder::every(10);
        sim.run_until(scenario.spec.duration.min(1.0), &mut recorder);
        let e = sim.energy();
        assert!(
            e.kinetic.is_finite() && e.potential.is_finite(),
            "{}",
            scenario.spec.id
        );
    }
}

#[test]
fn projectile_lands_at_the_textbook_range() {
    let (scene, answers) = build(
        "projectile",
        Params::default().set("speed", 12.0).set("angle", 40.0),
    );
    let flight = exact(&answers, "Time of flight");
    let (dt, focus) = (scene.dt, scene.focus);
    let mut sim = Simulation::new(scene.world, dt);
    sim.advance((flight / dt).floor() as usize);
    let x = sim.world().bodies().position(focus).x;
    assert!(relative(x, exact(&answers, "Range")) < 2e-3, "x = {x}");
}

#[test]
fn incline_block_rests_or_slides_as_predicted() {
    for (angle, expect_slide) in [(20.0, false), (40.0, true)] {
        let (scene, answers) = build("incline-friction", Params::default().set("angle", angle));
        let slides = exact(&answers, "Slides?") == 1.0;
        assert_eq!(slides, expect_slide);
        let a = exact(&answers, "Acceleration");
        let focus = scene.focus;
        let mut sim = Simulation::new(scene.world, scene.dt);
        let mut recorder = Recorder::every(1);
        sim.run_until(0.5, &mut recorder);
        let speed = sim.world().bodies().velocity(focus).length();
        if slides {
            assert!(
                relative(speed, a * sim.time()) < 1e-6,
                "speed {speed} vs {}",
                a * sim.time()
            );
        } else {
            assert!(speed < 1e-9, "should rest, speed {speed}");
        }
        let friction = recorder
            .force(focus, ForceLabel::Friction)
            .last()
            .unwrap()
            .length();
        assert!(
            relative(friction, exact(&answers, "Friction")) < 1e-6,
            "friction {friction}"
        );
    }
}

#[test]
fn atwood_and_table_scenarios_reproduce_tension() {
    let (scene, answers) = build("atwood", Params::default());
    let focus = scene.focus;
    let mut sim = Simulation::new(scene.world, scene.dt);
    let mut recorder = Recorder::every(1);
    sim.run_until(0.5, &mut recorder);
    let tension = recorder
        .force(focus, ForceLabel::Tension)
        .last()
        .unwrap()
        .length();
    assert!(relative(tension, exact(&answers, "Tension")) < 1e-8);

    for mu in [0.0, 0.1] {
        let (scene, answers) = build(
            "table-and-hanging-mass",
            Params::default().set("friction", mu),
        );
        let a = exact(&answers, "Acceleration");
        let focus = scene.focus;
        let mut sim = Simulation::new(scene.world, scene.dt);
        sim.advance(500);
        let speed = sim.world().bodies().velocity(focus).x;
        assert!(
            relative(speed, a * sim.time()) < 1e-6,
            "mu {mu}: speed {speed} vs {}",
            a * sim.time()
        );
    }
}

#[test]
fn collision_scenario_matches_closed_form() {
    let (scene, answers) = build("collision-1d", Params::default().set("restitution", 0.5));
    let (v1, v2) = (
        exact(&answers, "Final speed 1"),
        exact(&answers, "Final speed 2"),
    );
    let lost = exact(&answers, "Kinetic energy lost");
    let mut sim = Simulation::new(scene.world, scene.dt);
    sim.advance(3000);
    let bodies = sim.world().bodies();
    assert!(relative(bodies.velocity(BodyId::from_index(1)).x, v1) < 1e-8);
    assert!(relative(bodies.velocity(BodyId::from_index(2)).x, v2) < 1e-8);
    assert!(relative(sim.energy().dissipated, lost) < 1e-8);
}

#[test]
fn pendulum_scenario_period() {
    let (scene, answers) = build("simple-pendulum", Params::default().set("amplitude", 60.0));
    let period = exact(&answers, "Period, exact");
    let (dt, focus) = (scene.dt, scene.focus);
    let mut sim = Simulation::new(scene.world, dt);
    let mut recorder = Recorder::every(1);
    sim.run_until(3.0 * period, &mut recorder);
    let x = &recorder.body(focus).position;
    let crossings: Vec<f64> = (1..x.len())
        .filter(|&i| x[i - 1].x > 0.0 && x[i].x <= 0.0)
        .map(|i| recorder.time[i - 1] + dt * x[i - 1].x / (x[i - 1].x - x[i].x))
        .collect();
    let measured = (crossings[crossings.len() - 1] - crossings[0]) / (crossings.len() - 1) as f64;
    assert!(relative(measured, period) < 1e-5, "{measured} vs {period}");
}

#[test]
fn massive_pulley_scenarios_match_their_answers() {
    for (id, body) in [("atwood-massive", "m2"), ("flywheel", "mass")] {
        let (scene, answers) = build(id, Params::default());
        let a = exact(&answers, "Acceleration");
        let focus = scene.labels.iter().position(|l| l == body).unwrap();
        let mut sim = Simulation::new(scene.world, scene.dt);
        sim.advance(500);
        let v = sim
            .world()
            .bodies()
            .velocity(BodyId::from_index(focus))
            .y
            .abs();
        assert!(
            relative(v, a * sim.time()) < 1e-7,
            "{id}: v {v} vs {}",
            a * sim.time()
        );
    }
}

// ---------------------------------------------------------------- Gravitation (design 009)

const KM: f64 = 1000.0;

/// Steps before a run is given up on, whatever was asked: a test must never record without end.
const STEP_LIMIT: usize = 200_000;

/// Runs a scenario until its problem ends or `seconds` have passed, whichever is first, sampling every step.
fn run(id: &str, params: Params, seconds: f64) -> (Simulation, Recorder, Scene2) {
    let (scene, answers) = build(id, params);
    let focus = scene.focus;
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
    (sim, recorder, Scene2 { focus, answers })
}

struct Scene2 {
    focus: BodyId,
    answers: Vec<(String, f64)>,
}

#[test]
fn a_circular_orbit_closes_at_its_period_with_constant_speed() {
    for altitude in [400.0, 20000.0] {
        let params = Params::default().set("altitude", altitude);
        let period = 60.0 * exact(&build("circular-orbit", params.clone()).1, "Period");
        let (sim, recorder, scene) = run("circular-orbit", params, period);
        let record = recorder.body(scene.focus);
        let speed = KM * exact(&scene.answers, "Orbital speed");
        for v in &record.velocity {
            // A few thousand steps a turn: the discrete orbit is a circle to second order in the step.
            assert!(relative(v.length(), speed) < 1e-4, "{}", v.length());
        }
        let (start, end) = (record.position[0], *record.position.last().unwrap());
        // One step of the orbit is `speed · dt` long, so the end can only be compared to that much.
        assert!((end - start).length() < 1.5 * speed * sim.dt(), "{end:?}");
        let a = record.acceleration[record.acceleration.len() / 2].length();
        let expected = exact(&scene.answers, "Acceleration");
        assert!(relative(a, expected) < 1e-5, "a {a} vs {expected}");
        let energy = &recorder.energy;
        let drift = relative(energy.last().unwrap().conserved(), energy[0].conserved());
        assert!(drift < 1e-9, "energy drift {drift}");
    }
}

#[test]
fn an_elliptical_orbit_obeys_keplers_laws() {
    let (sim, recorder, scene) = run("elliptical-orbit", Params::default(), 1e9);
    let finished = sim.finished().expect("the scene ends after two turns");
    let period = 60.0 * exact(&scene.answers, "Period");
    assert!(relative(finished.time, 2.05 * period) < 1e-3);

    let record = recorder.body(scene.focus);
    let earth_radius = 6.371e6;
    let farthest = record
        .position
        .iter()
        .map(|p| p.length())
        .fold(0.0, f64::max);
    let expected = KM * exact(&scene.answers, "Farthest altitude") + earth_radius;
    assert!(
        relative(farthest, expected) < 1e-5,
        "apogee {farthest}, expected {expected}"
    );

    let slowest = record
        .velocity
        .iter()
        .map(|v| v.length())
        .fold(f64::INFINITY, f64::min);
    let expected = KM * exact(&scene.answers, "Speed at the farthest");
    assert!(
        relative(slowest, expected) < 1e-5,
        "slowest {slowest} vs {expected}"
    );

    // Second law: the area swept per unit time, r × v / 2, never changes.
    let areal = |i: usize| 0.5 * record.position[i].cross(record.velocity[i]);
    let first = areal(0);
    for i in (0..record.position.len()).step_by(97) {
        assert!(relative(areal(i), first) < 1e-10, "areal velocity at {i}");
    }

    // Third law: back at the nearest point after one period.
    let step = (period / sim.dt()).round() as usize;
    let gap = (record.position[step] - record.position[0]).length();
    assert!(
        gap < 2.0 * record.velocity[0].length() * sim.dt(),
        "gap {gap}"
    );
}

#[test]
fn a_vertical_launch_returns_below_escape_speed_and_leaves_above_it() {
    let earth_radius = 6.371e6;
    let (sim, recorder, scene) = run("escape-velocity", Params::default().set("speed", 6.0), 1e9);
    assert_eq!(exact(&scene.answers, "Escapes?"), 0.0);
    let record = recorder.body(scene.focus);
    let top = record.position.iter().map(|p| p.y).fold(0.0, f64::max) - earth_radius;
    let expected = KM * exact(&scene.answers, "Maximum height");
    assert!(
        relative(top, expected) < 1e-5,
        "rose {top}, expected {expected}"
    );
    let finished = sim.finished().expect("it comes back down");
    assert!(matches!(
        finished.trigger,
        soliton_mechanics::Trigger::Contact(..)
    ));

    let escape = exact(&scene.answers, "Escape speed");
    assert!(relative(escape, 11.18) < 2e-3, "escape speed {escape} km/s");

    let (sim, _, scene) = run("escape-velocity", Params::default().set("speed", 12.0), 1e9);
    assert_eq!(exact(&scene.answers, "Escapes?"), 1.0);
    let bodies = sim.world().bodies();
    let (r, v) = (
        bodies.position(scene.focus).length(),
        bodies.velocity(scene.focus).length(),
    );
    assert!(r > 10.0 * earth_radius, "still at {r}");
    // Energy conservation gives the speed at any distance; far away it tends to the answer.
    let far = KM * exact(&scene.answers, "Speed left when far away");
    let here = (far * far + 2.0 * 6.674e-11 * 5.972e24 / r).sqrt();
    assert!(relative(v, here) < 1e-5, "speed {v}, expected {here}");
}

#[test]
fn two_free_bodies_orbit_their_common_centre() {
    use soliton_mechanics::{Body, Gravitation, Vec2, World};
    // Equal masses a distance d apart, each circling the midpoint: v² = G m / (2 d).
    let (g, m, d): (f64, f64, f64) = (1.0, 4.0, 2.0);
    let v = (g * m / (2.0 * d)).sqrt();
    let mut world = World::new(Vec2::ZERO);
    let a = world.add(Body::particle(m).at(-0.5 * d, 0.0).velocity(0.0, -v));
    let b = world.add(Body::particle(m).at(0.5 * d, 0.0).velocity(0.0, v));
    world.add_force(Gravitation::new(g));
    let mut sim = Simulation::new(world, 1e-3);
    let start = sim.energy().conserved();
    let period = std::f64::consts::TAU * 0.5 * d / v;
    sim.advance((period / 1e-3).round() as usize);
    let bodies = sim.world().bodies();
    // The period is not a whole number of steps, so the body is within one step of where it began.
    assert!((bodies.position(a) - Vec2::new(-0.5 * d, 0.0)).length() < v * 1e-3);
    assert!(relative((bodies.position(a) - bodies.position(b)).length(), d) < 1e-7);
    assert!(sim.momentum().length() < 1e-12);
    assert!(relative(sim.energy().conserved(), start) < 1e-9);
}

// ---------------------------------------------------------------- Damped and driven oscillators

#[test]
fn a_damped_oscillator_follows_its_envelope() {
    let (m, k, b, x0) = (1.0, 20.0, 0.5, 0.8);
    let (sim, recorder, scene) = run("damped-oscillator", Params::default(), 6.0);
    let record = recorder.body(scene.focus);
    for i in (0..record.position.len()).step_by(250) {
        let t = recorder.time[i];
        let x = record.position[i].x - 2.0;
        let expected = soliton_mechanics::exact::damped_oscillator(x0, m, k, b, t);
        assert!((x - expected).abs() < 1e-5, "t = {t}: {x} vs {expected}");
    }
    // What the spring lost, the drag took.
    let energy = sim.energy();
    assert!(energy.dissipated > 0.9 * 0.5 * k * x0 * x0);
    let ledger = relative(energy.conserved(), recorder.energy[0].conserved());
    assert!(ledger < 1e-5, "ledger drift {ledger}");

    let gamma = b / (2.0 * m);
    let damped = std::f64::consts::TAU / (k / m - gamma * gamma).sqrt();
    assert!(relative(exact(&scene.answers, "Period (s)"), damped) < 1e-12);
    assert!(
        relative(
            exact(&scene.answers, "Time for the swing to halve"),
            2f64.ln() / gamma
        ) < 1e-12
    );

    let (_, answers) = build("damped-oscillator", Params::default().set("damping", 10.0));
    assert_eq!(exact(&answers, "Oscillates?"), 0.0);
    assert!(!answers.iter().any(|(name, _)| name == "Period (s)"));
}

#[test]
fn a_driven_oscillator_settles_to_the_steady_amplitude() {
    for frequency in [2.0, 4.0, 7.0] {
        let (sim, recorder, scene) = run(
            "driven-oscillator",
            Params::default().set("frequency", frequency),
            1e9,
        );
        assert!(sim.finished().is_some());
        let record = recorder.body(scene.focus);
        // The swing over the last two cycles of the drive.
        let cycles = (2.0 * std::f64::consts::TAU / frequency / sim.dt()) as usize;
        let tail = &record.position[record.position.len() - cycles..];
        let (low, high) = tail
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| {
                (lo.min(p.x), hi.max(p.x))
            });
        let amplitude = 0.5 * (high - low);
        let expected = exact(&scene.answers, "Steady amplitude");
        assert!(
            relative(amplitude, expected) < 2e-3,
            "ω = {frequency}: {amplitude} vs {expected}"
        );
        // In the steady state the drive puts in exactly what the drag takes out.
        let energy = sim.energy();
        assert!(
            relative(
                energy.conserved(),
                recorder.energy[0].conserved().abs().max(1.0)
            ) < 1.0
        );
    }
    let (_, answers) = build("driven-oscillator", Params::default());
    let peak = exact(&answers, "Largest steady amplitude");
    assert!(peak > exact(&answers, "Steady amplitude"));
}

#[test]
fn coupled_oscillators_have_two_modes_and_trade_their_motion() {
    let (m, k, coupling, a): (f64, f64, f64, f64) = (1.0, 20.0, 2.0, 0.4);
    let (together, against) = ((k / m).sqrt(), ((k + 2.0 * coupling) / m).sqrt());
    let left = |r: &Recorder, scene: &Scene2, i: usize| r.body(scene.focus).position[i].x - 2.0;

    // Started together or exactly against each other, each block is a plain oscillator at one frequency.
    for (second, omega) in [(a, together), (-a, against)] {
        let params = Params::default().set("first", a).set("second", second);
        let (_, recorder, scene) = run("coupled-oscillators", params, 6.0);
        for i in (0..recorder.time.len()).step_by(500) {
            let expected = a * (omega * recorder.time[i]).cos();
            let x = left(&recorder, &scene, i);
            assert!(
                (x - expected).abs() < 2e-5,
                "ω = {omega}: {x} vs {expected}"
            );
        }
    }

    // Started with one block alone, the motion is both modes at once: beats.
    let (sim, recorder, scene) = run("coupled-oscillators", Params::default(), 1e9);
    assert!(sim.finished().is_some());
    for i in (0..recorder.time.len()).step_by(500) {
        let t = recorder.time[i];
        let expected = 0.5 * a * ((together * t).cos() + (against * t).cos());
        let x = left(&recorder, &scene, i);
        assert!((x - expected).abs() < 5e-5, "t = {t}: {x} vs {expected}");
    }
    let handover = exact(&scene.answers, "Time for the motion to cross over");
    assert!(relative(handover, std::f64::consts::PI / (against - together)) < 1e-12);
    let energy = &recorder.energy;
    // Verlet's energy wobbles about the true value by (ω dt)²/8 of the oscillation's energy and does not drift.
    let wobble = relative(energy.last().unwrap().conserved(), energy[0].conserved());
    assert!(wobble < 1e-5, "energy off by {wobble}");
}
