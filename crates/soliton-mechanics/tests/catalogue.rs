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
