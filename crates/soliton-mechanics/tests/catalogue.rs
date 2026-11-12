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

// ---------------------------------------------------------------- The rest of the class 11 catalogue

#[test]
fn a_sliding_block_stops_where_the_kinematics_says() {
    let (sim, _, scene) = run("stopping-distance", Params::default(), 1e9);
    let finished = sim.finished().expect("it comes to rest");
    let block = sim.world().bodies().position(scene.focus);
    assert!(
        relative(block.x, exact(&scene.answers, "Stopping distance")) < 1e-6,
        "{}",
        block.x
    );
    assert!((finished.time - exact(&scene.answers, "Time to stop")).abs() < 2e-3);
    // v² = u² − 2 μ g s: twice the speed, four times the distance.
    let (_, fast) = build("stopping-distance", Params::default().set("speed", 20.0));
    assert!(
        relative(
            exact(&fast, "Stopping distance"),
            4.0 * exact(&scene.answers, "Stopping distance")
        ) < 1e-12
    );
}

#[test]
fn a_projectile_lands_on_the_slope_at_the_textbook_range() {
    for (elevation, slope) in [(30.0, 25.0), (50.0, 10.0), (15.0, 40.0)] {
        let params = Params::default()
            .set("elevation", elevation)
            .set("slope", slope);
        let (sim, recorder, scene) = run("projectile-on-incline", params, 1e9);
        let finished = sim.finished().expect("it lands");
        assert!(matches!(
            finished.trigger,
            soliton_mechanics::Trigger::Contact(..)
        ));
        // The ball starts 2 mm clear of the slope so that it begins free; crossing that takes a few ten-thousandths
        // of a second more than the formula for a ball that starts on it.
        let flight = exact(&scene.answers, "Time of flight");
        assert!(
            relative(finished.time, flight) < 5e-4,
            "flew for {}, expected {flight}",
            finished.time
        );
        let record = recorder.body(scene.focus);
        let flown = (*record.position.last().unwrap() - record.position[0]).length();
        let range = exact(&scene.answers, "Range along the slope");
        // The run stops two steps after the landing; in that time the ball has moved a few centimetres at most.
        assert!(
            (flown - range).abs() < 0.08,
            "slope {slope}: flew {flown}, expected {range}"
        );
        assert!(range <= exact(&scene.answers, "Greatest range") * (1.0 + 1e-12));
    }
    // Aimed halfway between the slope and the vertical, the range is the greatest there is.
    let (_, best) = build(
        "projectile-on-incline",
        Params::default().set("slope", 30.0).set("elevation", 30.0),
    );
    assert!(
        relative(
            exact(&best, "Range along the slope"),
            exact(&best, "Greatest range")
        ) < 1e-12
    );
}

#[test]
fn a_whirled_stone_keeps_its_speed_and_the_string_supplies_the_centripetal_force() {
    let (sim, recorder, scene) = run("circular-motion", Params::default(), 1e9);
    let period = exact(&scene.answers, "Period");
    assert!(relative(sim.finished().unwrap().time, 2.05 * period) < 1e-3);
    let record = recorder.body(scene.focus);
    let tension = recorder.force(scene.focus, ForceLabel::Tension);
    for i in (10..record.velocity.len()).step_by(211) {
        assert!(
            relative(record.velocity[i].length(), 3.0) < 1e-9,
            "speed at {i}"
        );
        assert!(
            relative(record.position[i].length(), 1.0) < 1e-12,
            "radius at {i}"
        );
        assert!(
            relative(tension[i].length(), exact(&scene.answers, "Tension")) < 1e-5,
            "tension at {i}"
        );
        assert!(
            relative(
                record.acceleration[i].length(),
                exact(&scene.answers, "Centripetal acceleration")
            ) < 1e-5
        );
        // The pull is along the string, towards the centre.
        assert!(tension[i].dot(record.position[i]) < 0.0);
    }
    let step = (period / sim.dt()).round() as usize;
    assert!((record.position[step] - record.position[0]).length() < 2.0 * 3.0 * sim.dt());
}

#[test]
fn the_string_between_two_blocks_carries_a_fixed_share_of_the_pull() {
    for friction in [0.0, 0.15] {
        let params = Params::default().set("friction", friction);
        let (sim, recorder, scene) = run("connected-blocks", params, 1.0);
        let a = recorder.body(scene.focus).acceleration.last().unwrap().x;
        assert!(
            relative(a, exact(&scene.answers, "Acceleration")) < 1e-8,
            "μ = {friction}: a = {a}"
        );
        let tension = recorder
            .force(scene.focus, ForceLabel::Tension)
            .last()
            .unwrap()
            .length();
        // 2 kg behind, 3 kg in front, 10 N: the string carries 2/5 of the pull whatever the floor is like.
        assert!(
            relative(tension, 4.0) < 1e-8,
            "μ = {friction}: T = {tension}"
        );
        assert!(relative(tension, exact(&scene.answers, "Tension")) < 1e-12);
        assert!(sim.finished().is_none());
    }
    let (_, stuck) = build("connected-blocks", Params::default().set("friction", 0.5));
    assert_eq!(exact(&stuck, "Moves?"), 0.0);
    assert!(relative(exact(&stuck, "Pull needed"), 0.5 * 5.0 * 9.8) < 1e-12);
}

#[test]
fn the_floor_of_a_lift_pushes_with_the_apparent_weight() {
    for acceleration in [2.0, 0.0, -3.0, -9.8] {
        let params = Params::default().set("acceleration", acceleration);
        let (sim, recorder, scene) = run("lift", params, 1.0);
        let normal = recorder
            .force(scene.focus, ForceLabel::Normal)
            .last()
            .unwrap()
            .y;
        let expected = 60.0 * (9.8 + acceleration);
        assert!(
            (normal - expected).abs() < 1e-6,
            "a = {acceleration}: N = {normal}, expected {expected}"
        );
        assert!((exact(&scene.answers, "Apparent weight") - expected).abs() < 1e-9);
        let a = recorder.body(scene.focus).acceleration.last().unwrap().y;
        assert!(
            (a - acceleration).abs() < 1e-8,
            "the person accelerates at {a}"
        );
        // The person rides on the floor: 0.6 m above its centre throughout.
        let bodies = sim.world().bodies();
        let floor = soliton_mechanics::BodyId::from_index(0);
        assert!((bodies.position(scene.focus).y - bodies.position(floor).y - 0.6).abs() < 1e-6);
    }
}

#[test]
fn gun_and_shell_leave_with_equal_and_opposite_momentum() {
    let (sim, _, scene) = run("recoil", Params::default(), 1.0);
    let bodies = sim.world().bodies();
    let shell = soliton_mechanics::BodyId::from_index(2);
    assert!(
        relative(bodies.velocity(shell).x, 20.0) < 1e-10,
        "{:?}",
        bodies.velocity(shell)
    );
    let recoil = -bodies.velocity(scene.focus).x;
    assert!(
        relative(recoil, exact(&scene.answers, "Recoil speed")) < 1e-10,
        "recoil {recoil}"
    );
    assert!(
        sim.momentum().x.abs() < 1e-9,
        "total momentum {}",
        sim.momentum().x
    );
    // The charge's work is all the kinetic energy there is, and the ledger has it as work done on the system.
    let energy = sim.energy();
    let kinetic = 0.5 * 0.5 * 400.0 + 0.5 * 5.0 * recoil * recoil;
    assert!(relative(energy.kinetic, kinetic) < 1e-9);
    assert!(
        relative(energy.external_work, kinetic) < 1e-6,
        "work {}",
        energy.external_work
    );
    let share = 100.0 * (0.5 * 0.5 * 400.0) / kinetic;
    assert!(relative(share, exact(&scene.answers, "Share of the energy")) < 1e-9);
}

#[test]
fn a_spring_takes_all_of_a_blocks_kinetic_energy_and_gives_it_back() {
    let (sim, recorder, scene) = run("spring-compression", Params::default(), 1e9);
    assert!(sim.finished().is_some());
    let block = recorder.body(scene.focus);
    // The block's near face meets the spring's free end 2 m from the wall. The spring comes on part-way through
    // a step, which costs about one part in 100 000.
    let nearest = block
        .position
        .iter()
        .map(|p| p.x - 0.3)
        .fold(f64::INFINITY, f64::min);
    let expected = exact(&scene.answers, "Greatest compression");
    assert!(
        relative(2.0 - nearest, expected) < 1e-4,
        "squeezed {}, expected {expected}",
        2.0 - nearest
    );
    let back = sim.world().bodies().velocity(scene.focus).x;
    assert!(
        relative(back, exact(&scene.answers, "Speed it comes back with")) < 1e-4,
        "came back at {back}"
    );
    let spring = recorder.force(scene.focus, ForceLabel::Spring);
    let hardest = spring.iter().map(|f| f.length()).fold(0.0, f64::max);
    assert!(relative(hardest, exact(&scene.answers, "Greatest force")) < 1e-4);
    // Before it arrives and after it leaves, a spring with a free end does nothing at all.
    assert_eq!(spring[10].length(), 0.0);
    assert_eq!(spring.last().unwrap().length(), 0.0);
    let energy = &recorder.energy;
    assert!(relative(energy.last().unwrap().conserved(), energy[0].conserved()) < 1e-4);
    assert!(
        sim.events().is_empty(),
        "nothing touches anything: {:?}",
        sim.events()
    );
}

#[test]
fn a_dropped_ball_squeezes_a_spring_past_where_it_would_rest() {
    for (height, stiffness) in [(1.0, 500.0), (0.3, 200.0), (2.5, 1500.0)] {
        let params = Params::default()
            .set("height", height)
            .set("stiffness", stiffness);
        let (sim, recorder, scene) = run("ball-on-spring", params, 1e9);
        let ball = recorder.body(scene.focus);
        // The ball's lowest point meets the spring's free end 1.2 m above the ground.
        let lowest = ball
            .position
            .iter()
            .map(|p| p.y - 0.15)
            .fold(f64::INFINITY, f64::min);
        let expected = exact(&scene.answers, "Greatest compression");
        assert!(
            relative(1.2 - lowest, expected) < 1e-4,
            "h = {height}: squeezed {}, expected {expected}",
            1.2 - lowest
        );
        assert!(expected > 2.0 * exact(&scene.answers, "Compression it would rest at"));
        // It leaves the spring as fast as it arrived, and the scene ends a moment after it has.
        let finished = sim
            .finished()
            .expect("the scene ends once the ball has left the spring");
        let leaving = ball.velocity.last().unwrap().y;
        assert!(
            leaving > 0.0 && ball.position.last().unwrap().y - 0.15 > 1.2,
            "ended at {}",
            finished.time
        );
        let energy = &recorder.energy;
        assert!(relative(energy.last().unwrap().conserved(), energy[0].conserved()) < 1e-4);
    }
}

#[test]
fn equal_discs_leave_a_glancing_collision_at_right_angles() {
    for offset in [0.2, 0.5, 0.9] {
        let (sim, _, scene) = run(
            "oblique-collision",
            Params::default().set("offset", offset),
            1e9,
        );
        let bodies = sim.world().bodies();
        let (incoming, struck) = (
            bodies.velocity(scene.focus),
            bodies.velocity(soliton_mechanics::BodyId::from_index(1)),
        );
        // What holds to round-off whatever the geometry: momentum, kinetic energy, and so the right angle.
        assert!(
            incoming.dot(struck).abs() < 1e-9,
            "offset {offset}: not at right angles"
        );
        assert!((sim.momentum() - soliton_mechanics::Vec2::new(4.0, 0.0)).length() < 1e-9);
        assert!(relative(sim.energy().kinetic, 8.0) < 1e-9);
        // How the speed is shared depends on exactly where they touch. The engine finds a contact up to one step
        // late, with the discs a few millimetres into each other, which turns the line of centres by a fraction
        // of a degree: more when they barely graze.
        let grazing = if offset > 0.8 { 3.0 } else { 1.0 };
        let angle = (-struck.y).atan2(struck.x).to_degrees();
        assert!(
            (angle - exact(&scene.answers, "Angle of the struck disc")).abs() < 0.5 * grazing,
            "offset {offset}: angle {angle}"
        );
        assert!(
            relative(
                struck.length(),
                exact(&scene.answers, "Speed of the struck disc")
            ) < 0.01 * grazing
        );
        assert!(
            (incoming.length() - exact(&scene.answers, "Speed of the incoming disc")).abs()
                < 0.03 * grazing
        );
    }
}

/// The time between successive moments a recorded velocity turns from positive to negative.
fn period_from(times: &[f64], velocity: &[f64]) -> f64 {
    let mut turns = Vec::new();
    for i in 1..velocity.len() {
        if velocity[i - 1] > 0.0 && velocity[i] <= 0.0 {
            let fraction = velocity[i - 1] / (velocity[i - 1] - velocity[i]);
            turns.push(times[i - 1] + fraction * (times[i] - times[i - 1]));
        }
    }
    assert!(
        turns.len() >= 2,
        "needs two turns to measure a period, found {}",
        turns.len()
    );
    (turns[turns.len() - 1] - turns[0]) / (turns.len() - 1) as f64
}

#[test]
fn a_steady_torque_spins_a_pivoted_rod_up_at_a_constant_rate() {
    let (sim, recorder, scene) = run("torque-on-rod", Params::default(), 1e9);
    let finished = sim.finished().unwrap();
    assert_eq!(finished.time, 2.0);
    let rod = recorder.body(scene.focus);
    let step = (2.0 / sim.dt()).round() as usize;
    // 4 N·m on 2 kg and 2 m: I = 8/3, α = 1.5 rad/s², ω(2 s) = 3 rad/s, θ(2 s) = 3 rad.
    assert!(relative(exact(&scene.answers, "Angular acceleration"), 1.5) < 1e-12);
    assert!(
        relative(rod.angular_velocity[step], 3.0) < 1e-8,
        "ω = {}",
        rod.angular_velocity[step]
    );
    // The pin that holds the pivot is kept by projection, which is second order: a part in a million here.
    assert!(
        relative(rod.angle[step], 3.0) < 1e-5,
        "θ = {}",
        rod.angle[step]
    );
    // The rate the speed grows at, between the first and the second second.
    let alpha = (rod.angular_velocity[step] - rod.angular_velocity[step / 2]) / 1.0;
    assert!(relative(alpha, 1.5) < 1e-8, "α = {alpha}");
    // The acceleration the engine reports at an instant includes the pivot's reaction, which for a body turning
    // about a pin is right only to first order in ω dt: a third of a percent here. The motion does not use it.
    assert!(relative(rod.angular_acceleration[step / 2], 1.5) < 1e-2);
    assert!(
        relative(
            exact(&scene.answers, "Turns made"),
            3.0 / std::f64::consts::TAU
        ) < 1e-12
    );
    // The work the torque does, τθ, is the rod's kinetic energy, ½ I ω².
    let energy = recorder.energy[step];
    assert!(
        relative(energy.external_work, 12.0) < 1e-6,
        "work {}",
        energy.external_work
    );
    assert!(
        relative(energy.kinetic, 12.0) < 1e-8,
        "kinetic {}",
        energy.kinetic
    );
}

#[test]
fn gravity_shifts_a_vertical_spring_and_leaves_its_period_alone() {
    let (_, recorder, scene) = run("vertical-spring", Params::default(), 1e9);
    let mass = recorder.body(scene.focus);
    let y: Vec<f64> = mass.position.iter().map(|p| p.y).collect();
    let vy: Vec<f64> = mass.velocity.iter().map(|v| v.y).collect();
    let dropped = y[0] - y.iter().copied().fold(f64::INFINITY, f64::min);
    assert!(
        relative(dropped, exact(&scene.answers, "Lowest point")) < 1e-5,
        "dropped {dropped}"
    );
    assert!(
        relative(
            dropped,
            2.0 * exact(&scene.answers, "Stretch it would hang at")
        ) < 1e-5
    );
    let period = period_from(&recorder.time, &vy);
    assert!(
        relative(period, exact(&scene.answers, "Period")) < 1e-5,
        "period {period}"
    );
    // The same mass and spring on a level table have the same period.
    let (_, level) = build(
        "spring-mass",
        Params::default().set("mass", 1.0).set("stiffness", 40.0),
    );
    assert!(relative(exact(&level, "Period"), exact(&scene.answers, "Period")) < 1e-12);
    let fastest = vy.iter().map(|v| v.abs()).fold(0.0, f64::max);
    assert!(relative(fastest, exact(&scene.answers, "Greatest speed")) < 1e-5);
}

#[test]
fn springs_in_series_soften_and_springs_in_parallel_stiffen() {
    for (id, effective, tolerance) in [
        ("springs-in-series", 20.0, 2e-3),
        ("springs-in-parallel", 90.0, 1e-5),
    ] {
        let (_, recorder, scene) = run(id, Params::default(), 1e9);
        // 30 N/m and 60 N/m: 20 N/m end to end, 90 N/m side by side.
        assert!(
            relative(
                exact(&scene.answers, "Effective spring constant"),
                effective
            ) < 1e-12
        );
        let vx: Vec<f64> = recorder
            .body(scene.focus)
            .velocity
            .iter()
            .map(|v| v.x)
            .collect();
        let period = period_from(&recorder.time, &vx);
        let expected = exact(&scene.answers, "Period");
        // In series the join between the springs has a little mass of its own, a five-hundredth of the block's.
        assert!(
            relative(period, expected) < tolerance,
            "{id}: period {period}, expected {expected}"
        );
        assert!(relative(expected, std::f64::consts::TAU / effective.sqrt()) < 1e-12);
        let energy = &recorder.energy;
        assert!(
            relative(energy.last().unwrap().conserved(), energy[0].conserved()) < 1e-5,
            "{id}: energy"
        );
    }
    let (_, series) = build("springs-in-series", Params::default());
    let (first, second) = (
        exact(&series, "Stretch of the first"),
        exact(&series, "Stretch of the second"),
    );
    // The same force through both: k₁ x₁ = k₂ x₂.
    assert!(relative(30.0 * first, 60.0 * second) < 1e-12);
}
