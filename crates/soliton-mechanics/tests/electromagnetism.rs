//! Charges and fields (design 011): each class 12 scenario against its textbook answer, and the conservation
//! laws that should hold while it runs.

use soliton_mechanics::scenario::{find, Params};
use soliton_mechanics::scene::SceneFile;
use soliton_mechanics::{
    Body, BodyId, Distribution, Electrostatics, ForceLabel, MagneticField, Recorder, Simulate,
    Simulation, Vec2, World, COULOMB,
};

const STEP_LIMIT: usize = 400_000;

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

    fn drift(&self) -> f64 {
        let energy = &self.recorder.energy;
        let scale = energy
            .iter()
            .map(|e| e.kinetic.abs().max(e.potential.abs()))
            .fold(0.0, f64::max);
        (energy.last().unwrap().conserved() - energy[0].conserved()).abs() / scale
    }
}

/// Runs a scenario until its problem ends, or for `seconds` if that comes first, sampling every step.
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

#[test]
fn like_charges_repel_with_coulombs_force_and_trade_potential_for_kinetic_energy() {
    let r = run("coulomb-force", Params::default(), f64::INFINITY);
    let record = r.recorder.body(r.focus);
    let force = r.recorder.force(r.focus, ForceLabel::Electric)[0];
    assert!(relative(force.x * 1e3, r.answer("Force at the start")) < 1e-12);
    assert!(
        relative(
            record.acceleration[0].x,
            r.answer("Acceleration at the start")
        ) < 1e-12
    );
    let start = record.position[0].x;
    let at_twice = record
        .position
        .iter()
        .position(|p| p.x >= 2.0 * start)
        .unwrap();
    let speed = record.velocity[at_twice].x;
    assert!(
        relative(speed, r.answer("Speed at twice")) < 2e-3,
        "{speed}"
    );
    assert!(r.drift() < 1e-7, "energy drift {}", r.drift());
}

#[test]
fn the_force_falls_as_the_inverse_square() {
    let force = |separation: f64| {
        let r = run(
            "coulomb-force",
            Params::default().set("separation", separation),
            0.0,
        );
        r.answer("Force at the start")
    };
    assert!(relative(force(0.5) / force(1.0), 4.0) < 1e-12);
}

#[test]
fn field_and_potential_of_two_charges_superpose() {
    let params = Params::default().set("x", 0.4).set("y", 0.7);
    let r = run("field-of-two-charges", params, 0.0);
    // The force on the test charge, divided by its charge, is the field the other two make where it sits.
    let force = r.recorder.force(r.focus, ForceLabel::Electric)[0];
    assert!(relative(force.length() * 1e6, r.answer("Force on the test charge")) < 1e-12);
    assert!(relative(force.length() / 1e-9 / 1e3, r.answer("Field at")) < 1e-12);
    // On the line midway between equal and opposite charges the potential is zero and the field points
    // straight across, from the positive charge towards the negative. The test charge itself, a thousandth of
    // either, is the only thing that spoils this.
    let world = r.sim.world();
    let midway = world.electric_field(Vec2::new(0.0, -1.3), 0.0);
    assert!(
        midway.x > 1e4 && midway.y.abs() < 1e-3 * midway.x,
        "{midway:?}"
    );
    assert!(world.electric_potential(Vec2::new(0.0, -1.3)).abs() < 10.0);
    // Close to the positive charge the potential is that of the charge alone, kq/r.
    let near = world.electric_potential(Vec2::new(-1.0, 0.01));
    assert!(relative(near, COULOMB * 4e-6 / 0.01) < 0.01, "{near}");
}

#[test]
fn field_lines_run_from_the_positive_charge_to_the_negative() {
    let scene = find("field-of-two-charges")
        .unwrap()
        .build(&Params::default());
    let reach = 6.0;
    let lines = scene.world.field_lines(12, Vec2::ZERO, reach, 0.0);
    let (positive, negative) = (Vec2::new(-1.0, 0.0), Vec2::new(1.0, 0.0));
    let leaving: Vec<&Vec<Vec2>> = lines
        .iter()
        .filter(|line| (line[0] - positive).length() < 1e-9)
        .collect();
    assert_eq!(leaving.len(), 12);
    let mut landed = 0;
    for line in &leaving {
        let end = *line.last().unwrap();
        // The widest loops leave the picture before they come back; every other line ends on the negative charge.
        if (end - negative).length() < 1e-9 {
            landed += 1;
        } else {
            assert!(end.length() > reach, "a line stopped in mid-air at {end:?}");
        }
    }
    assert!(
        landed >= 6,
        "{landed} of 12 lines reached the negative charge"
    );
    // The far halves of those loops come back in to the negative charge, and nothing is drawn twice.
    assert_eq!(lines.len(), 12 + (12 - landed));
    for line in lines
        .iter()
        .filter(|line| (line[0] - positive).length() > 1e-9)
    {
        assert!((*line.last().unwrap() - negative).length() < 1e-9);
    }
}

#[test]
fn a_charged_bob_swings_to_twice_the_angle_it_would_hang_at() {
    let r = run("charged-pendulum", Params::default(), 6.0);
    let record = r.recorder.body(r.focus);
    let furthest = record
        .position
        .iter()
        .map(|p| p.x.atan2(-p.y).to_degrees())
        .fold(0.0, f64::max);
    assert!(
        relative(furthest, r.answer("Furthest it swings")) < 1e-4,
        "{furthest}°"
    );
    assert!(r.drift() < 1e-6, "energy drift {}", r.drift());
}

#[test]
fn a_charged_bob_released_where_it_would_hang_stays_there() {
    let scene = find("charged-pendulum").unwrap().build(&Params::default());
    let rest = scene
        .exact
        .iter()
        .find(|(n, _)| n.starts_with("Angle"))
        .unwrap()
        .1
        .to_radians();
    let tension = scene
        .exact
        .iter()
        .find(|(n, _)| n.starts_with("Tension"))
        .unwrap()
        .1;
    let mut file = find("charged-pendulum")
        .unwrap()
        .scene_file(&Params::default());
    file.bodies[0].position = [rest.sin(), -rest.cos()];
    let scene = SceneFile::from_json(&file.to_json())
        .unwrap()
        .build()
        .unwrap();
    let focus = scene.focus;
    let mut sim = Simulation::new(scene.world, scene.dt);
    let mut recorder = Recorder::every(1);
    sim.run_until(1.0, &mut recorder);
    assert!(sim.world().bodies().velocity(focus).length() < 1e-9);
    let pull = recorder
        .force(focus, ForceLabel::Tension)
        .last()
        .unwrap()
        .length();
    assert!(relative(pull * 1e3, tension) < 1e-9, "{pull}");
}

#[test]
fn a_charge_between_plates_follows_a_parabola_and_then_a_straight_line() {
    for charge in [1.0, -2.0] {
        let r = run(
            "charge-between-plates",
            Params::default().set("charge", charge),
            f64::INFINITY,
        );
        assert!(r.sim.finished().is_some());
        let record = r.recorder.body(r.focus);
        let leaving = record.position.iter().position(|p| p.x >= 2.0).unwrap();
        let deflection = record.position[leaving].y;
        assert!(
            relative(deflection, r.answer("Deflection on leaving")) < 3e-3,
            "{deflection}"
        );
        let landed = record.position.last().unwrap().y;
        assert!(
            relative(landed, r.answer("Where it meets the screen")) < 3e-3,
            "{landed}"
        );
        // The field does not touch the speed along the plates.
        let across = record.velocity[leaving].x;
        assert!(relative(across, 4.0) < 1e-12);
    }
}

#[test]
fn a_drop_hangs_still_in_the_balancing_field_and_rises_in_a_stronger_one() {
    let balance = run("balanced-drop", Params::default(), 0.0).answer("Field that holds");
    let still = run(
        "balanced-drop",
        Params::default().set("field", balance),
        1.0,
    );
    assert!(still.sim.world().bodies().velocity(still.focus).length() < 1e-9);
    let rising = run("balanced-drop", Params::default(), 1.0);
    let a = rising.recorder.body(rising.focus).acceleration[10].y;
    assert!(
        relative(a, rising.answer("Acceleration, upward")) < 1e-9,
        "{a}"
    );
    assert!(rising.drift() < 1e-9);
}

#[test]
fn a_dipole_feels_a_torque_but_no_net_force() {
    let r = run(
        "dipole-in-field",
        Params::default().set("angle", 4.0),
        f64::INFINITY,
    );
    let (positive, negative) = (BodyId::from_index(0), BodyId::from_index(1));
    let (a, b) = (r.recorder.body(positive), r.recorder.body(negative));
    for (p, q) in a.position.iter().zip(&b.position) {
        assert!((*p + *q).length() < 1e-9, "the middle of the rod moved");
    }
    // The positive end crosses the field's direction going down, then comes back up: half a period apart.
    let crossings: Vec<f64> = a
        .position
        .windows(2)
        .zip(&r.recorder.time)
        .filter(|(w, _)| w[0].y > 0.0 && w[1].y <= 0.0 || w[0].y < 0.0 && w[1].y >= 0.0)
        .map(|(_, t)| *t)
        .collect();
    let period = 2.0 * (crossings[1] - crossings[0]);
    assert!(
        relative(period, r.answer("Period of small swings")) < 2e-3,
        "{period}"
    );
    assert!(r.drift() < 1e-6, "energy drift {}", r.drift());
}

#[test]
fn a_charge_fired_at_a_like_charge_turns_back_where_its_energy_is_all_potential() {
    let r = run("closest-approach", Params::default(), f64::INFINITY);
    let record = r.recorder.body(r.focus);
    let closest = record
        .position
        .iter()
        .map(|p| p.length())
        .fold(f64::INFINITY, f64::min);
    assert!(
        relative(closest, r.answer("Closest distance")) < 1e-5,
        "{closest}"
    );
    let back = record.velocity.last().unwrap().x;
    assert!(back > 0.0, "it should be on its way out again");
    assert!(r.drift() < 1e-7, "energy drift {}", r.drift());
}

#[test]
fn a_potential_difference_gives_the_same_energy_whatever_the_gap() {
    for gap in [0.5, 2.0] {
        let r = run(
            "accelerating-voltage",
            Params::default().set("gap", gap),
            f64::INFINITY,
        );
        let speed = r.sim.world().bodies().velocity(r.focus).x;
        assert!(
            relative(speed, r.answer("Speed on leaving")) < 1e-3,
            "{speed}"
        );
        // The supply's work is the ledger's external work, and all of it became kinetic energy.
        let energy = r.sim.energy();
        assert!(relative(energy.external_work, energy.kinetic) < 1e-6);
        assert!(
            relative(
                energy.external_work * 1e3,
                r.answer("Kinetic energy gained")
            ) < 2e-3
        );
    }
}

#[test]
fn a_magnetic_field_turns_a_charge_in_a_circle_without_changing_its_speed() {
    for (speed, field) in [(3.0, 1.0), (6.0, 2.5)] {
        let params = Params::default().set("speed", speed).set("field", field);
        let r = run("charge-in-magnetic-field", params, f64::INFINITY);
        let record = r.recorder.body(r.focus);
        let radius = r.answer("Radius");
        for (p, v) in record.position.iter().zip(&record.velocity) {
            assert!(relative(v.length(), speed) < 1e-12, "speed {}", v.length());
            assert!(relative(p.length(), radius) < 1e-5, "radius {}", p.length());
        }
        // Clockwise for a positive charge with the field out of the page.
        assert!(record.position[10].x > 0.0 && record.position[10].y < radius);
        let period = r.answer("Time for one turn");
        let back = r.recorder.time.iter().position(|&t| t >= period).unwrap();
        let gap = (record.position[back] - record.position[0]).length();
        assert!(gap < 2.0 * speed * r.sim.dt(), "missed the start by {gap}");
        let force = r.recorder.force(r.focus, ForceLabel::Magnetic)[5].length();
        assert!(relative(force * 1e6, r.answer("Magnetic force")) < 1e-9);
    }
}

#[test]
fn the_period_in_a_magnetic_field_does_not_depend_on_the_speed() {
    let period = |speed: f64| {
        run(
            "charge-in-magnetic-field",
            Params::default().set("speed", speed),
            0.0,
        )
        .answer("Time for one turn")
    };
    assert_eq!(period(1.0), period(7.0));
}

#[test]
fn only_one_speed_goes_straight_through_crossed_fields() {
    let land = |speed: f64| {
        let r = run(
            "velocity-selector",
            Params::default().set("speed", speed),
            f64::INFINITY,
        );
        assert!(
            r.sim.finished().is_some(),
            "speed {speed} never reached the screen"
        );
        r.sim.world().bodies().position(r.focus).y
    };
    assert!(
        land(4.0).abs() < 1e-9,
        "E/B should go straight: {}",
        land(4.0)
    );
    assert!(
        land(5.0) < -0.05,
        "a faster charge is bent by the magnetic force"
    );
    assert!(land(3.0) > 0.05, "a slower one by the electric force");
}

#[test]
fn a_cyclotron_speeds_a_charge_up_in_step_until_it_leaves_the_magnet() {
    let r = run("cyclotron", Params::default(), f64::INFINITY);
    let record = r.recorder.body(r.focus);
    let top = r.answer("Most speed");
    let speed = record.velocity.last().unwrap().length();
    // The last crossing may or may not happen before the orbit outgrows the magnet.
    assert!(
        speed > 0.8 * top && speed < 1.1 * top,
        "left at {speed}, formula {top}"
    );
    let outside = record.position.last().unwrap();
    assert!(
        outside.x.abs() > 2.0 || outside.y.abs() > 2.0,
        "still inside at {outside:?}"
    );
    // Successive upward crossings of the gap's middle are one turn apart, slow or fast.
    let turn = r.answer("Time for one turn");
    let crossings: Vec<f64> = record
        .position
        .windows(2)
        .zip(&r.recorder.time)
        .filter(|(w, _)| w[0].x < 0.0 && w[1].x >= 0.0 && w[0].x.hypot(w[0].y) > 0.05)
        .map(|(_, t)| *t)
        .collect();
    assert!(crossings.len() >= 3, "{} turns", crossings.len());
    for pair in crossings.windows(2).take(4) {
        assert!(
            relative(pair[1] - pair[0], turn) < 0.03,
            "turn took {}",
            pair[1] - pair[0]
        );
    }
    // All the energy came from the supply, a little less than qV at each crossing.
    let energy = r.sim.energy();
    assert!(relative(energy.external_work, energy.kinetic) < 1e-4);
}

#[test]
fn two_free_charges_conserve_momentum_and_energy() {
    let mut world = World::new(Vec2::ZERO);
    // Opposite charges going round each other, with no momentum between them.
    let a = world.add(
        Body::particle(0.002)
            .at(-0.5, 0.0)
            .velocity(0.0, 2.5)
            .charge(1e-6),
    );
    let b = world.add(
        Body::particle(0.005)
            .at(0.5, 0.0)
            .velocity(0.0, -1.0)
            .charge(-2e-6),
    );
    world.add_force(Electrostatics::default());
    let mut sim = Simulation::new(world, 1e-4);
    let (momentum, energy) = (sim.momentum(), sim.energy().conserved());
    sim.advance(20_000);
    assert!((sim.momentum() - momentum).length() < 1e-15);
    assert!(relative(sim.energy().conserved(), energy) < 1e-6);
    let apart = (sim.world().bodies().position(a) - sim.world().bodies().position(b)).length();
    assert!(
        apart > 0.5 && apart < 2.0,
        "they should still be in orbit, {apart} m apart"
    );
}

#[test]
fn a_magnetic_force_does_no_work() {
    let mut world = World::new(Vec2::new(0.0, -9.8));
    let id = world.add(Body::particle(1e-6).velocity(2.0, 1.0).charge(1e-6));
    world.add_force(MagneticField::uniform(1.5));
    let mut sim = Simulation::new(world, 1e-3);
    let start = sim.energy();
    sim.advance(5_000);
    let end = sim.energy();
    assert_eq!(end.external_work, 0.0);
    assert_eq!(end.dissipated, 0.0);
    // Gravity and the field together make the charge drift sideways; kinetic plus potential still holds, to
    // the second order of the step and against energies that have each grown tenfold.
    assert!(
        relative(end.mechanical(), start.mechanical()) < 1e-4,
        "{end:?}"
    );
    assert!(sim.world().bodies().position(id).x.abs() > 1.0);
}

#[test]
fn charge_and_fields_survive_a_round_trip_through_json() {
    for id in [
        "coulomb-force",
        "velocity-selector",
        "cyclotron",
        "dipole-in-field",
    ] {
        let file = find(id).unwrap().scene_file(&Params::default());
        let again = SceneFile::from_json(&file.to_json()).unwrap();
        assert_eq!(file, again, "{id}");
        assert!(again.build().is_ok());
    }
}

const EPSILON: f64 = 1.0 / (4.0 * std::f64::consts::PI * COULOMB);

fn flux_of(params: Params) -> (f64, f64) {
    let scenario = find("gauss-law").unwrap();
    let surface = scenario.scene_file(&params).surface.unwrap();
    let scene = scenario.build(&params);
    let centre = Vec2::new(surface.centre[0], surface.centre[1]);
    let expected = scene
        .exact
        .iter()
        .find(|(n, _)| n.starts_with("Flux"))
        .unwrap()
        .1;
    (
        scene.world.electric_flux(centre, surface.radius) / 1000.0,
        expected,
    )
}

#[test]
fn the_flux_out_of_a_surface_is_the_charge_inside_over_epsilon() {
    // Round the left charge alone, 4 μC.
    let (flux, expected) = flux_of(Params::default());
    assert!(relative(expected, 4e-6 / EPSILON / 1000.0) < 1e-12);
    assert!(relative(flux, expected) < 1e-3, "{flux} against {expected}");
    // Round both, 4 − 2 μC, with neither at the centre.
    let both = Params::default()
        .set("x", 0.3)
        .set("y", 0.4)
        .set("radius", 2.0);
    let (flux, expected) = flux_of(both);
    assert!(relative(flux, expected) < 1e-3, "{flux} against {expected}");
    // Round neither: what goes in comes out.
    let neither = Params::default()
        .set("x", 0.0)
        .set("y", 1.5)
        .set("radius", 0.6);
    let (flux, expected) = flux_of(neither);
    assert_eq!(expected, 0.0);
    assert!(flux.abs() < 1e-3 * 4e-6 / EPSILON / 1000.0, "{flux}");
}

#[test]
fn the_flux_counts_the_part_of_a_wire_or_a_shell_that_is_inside() {
    let mut world = World::new(Vec2::ZERO);
    world.add(Body::particle(1.0));
    world.add_force(
        Electrostatics::default()
            .with(Distribution::Wire {
                at: Vec2::new(0.3, 0.0),
                density: 2e-6,
            })
            .with(Distribution::Shell {
                centre: Vec2::new(5.0, 0.0),
                radius: 0.5,
                charge: 3e-6,
            }),
    );
    // A sphere of radius 1 cuts a chord 2√(1 − 0.3²) long out of a wire 0.3 from its centre.
    let chord = 2.0 * (1.0f64 - 0.09).sqrt();
    let wire = world.electric_flux(Vec2::ZERO, 1.0);
    assert!(relative(wire, 2e-6 * chord / EPSILON) < 2e-3, "{wire}");
    let shell = world.electric_flux(Vec2::new(5.1, 0.2), 1.0);
    assert!(relative(shell, 3e-6 / EPSILON) < 2e-3, "{shell}");
}

#[test]
fn a_charge_circles_a_wire_at_the_same_speed_at_any_distance() {
    let mut speeds = Vec::new();
    for radius in [0.5, 1.2] {
        let r = run(
            "charged-wire",
            Params::default().set("radius", radius),
            60.0,
        );
        let record = r.recorder.body(r.focus);
        for (p, v) in record.position.iter().zip(&record.velocity) {
            assert!(relative(p.length(), radius) < 1e-4, "left the circle");
            assert!(relative(v.length(), r.answer("Speed that keeps a circle")) < 1e-4);
        }
        assert!(r.drift() < 1e-6, "energy drifted by {}", r.drift());
        speeds.push(r.answer("Speed that keeps a circle"));
        // 2.2 turns were asked for.
        let turned = r.sim.time() / r.answer("Time for one turn");
        assert!(relative(turned, 2.2) < 1e-2, "{turned} turns");
    }
    assert_eq!(speeds[0], speeds[1]);
}

#[test]
fn a_sheet_pushes_equally_hard_at_any_distance() {
    for distance in [0.3, 1.5] {
        let r = run(
            "charged-sheet",
            Params::default().set("distance", distance),
            0.5,
        );
        let a = r.answer("Acceleration");
        let x = r.recorder.body(r.focus).position.last().unwrap().x;
        let t = r.sim.time();
        assert!(relative(x - distance, 0.5 * a * t * t) < 1e-6);
        assert!(r.drift() < 1e-9);
    }
    let field = run("charged-sheet", Params::default(), 0.01).answer("Field of the sheet");
    assert!(relative(field, 0.5e-6 / (2.0 * EPSILON) / 1000.0) < 1e-12);
}

#[test]
fn an_opposite_charge_goes_to_and_fro_through_a_sheet() {
    let r = run("charged-sheet", Params::default().set("charge", -1.0), 60.0);
    let x: Vec<f64> = r
        .recorder
        .body(r.focus)
        .position
        .iter()
        .map(|p| p.x)
        .collect();
    let far = x.iter().cloned().fold(f64::INFINITY, f64::min);
    assert!(
        relative(far, -0.5) < 1e-2,
        "it went through as far as {far}"
    );
    // Back at the start after one whole trip, and again after two.
    let period = r.answer("Time to go through and come back");
    let index = (period / r.sim.dt()).round() as usize;
    assert!(
        (x[index] - 0.5).abs() < 0.01,
        "at {} after one trip",
        x[index]
    );
    assert!(r.drift() < 1e-2, "energy drifted by {}", r.drift());
}

#[test]
fn nothing_pushes_on_a_charge_inside_a_shell_and_a_point_charge_does_outside() {
    let r = run("charged-shell", Params::default(), 60.0);
    let record = r.recorder.body(r.focus);
    let inside = r.answer("Time it spends inside");
    let (dt, radius, big, q, m) = (r.sim.dt(), 0.8, 4e-6, 1e-6, 0.005);
    let before = (0.95 * inside / dt) as usize;
    assert!((record.velocity[before] - Vec2::new(2.0, 0.0)).length() < 1e-12);
    assert!(
        relative(
            record.position[(inside / dt).round() as usize].length(),
            radius
        ) < 1e-2
    );
    let (end, speed) = (
        record.position.last().unwrap().length(),
        record.velocity.last().unwrap().length(),
    );
    assert!(end > 1.5 * radius);
    let expected = (4.0 + 2.0 * COULOMB * big * q / m * (1.0 / radius - 1.0 / end)).sqrt();
    assert!(
        relative(speed, expected) < 1e-3,
        "{speed} against {expected}"
    );
    assert!(speed < r.answer("Speed far away"));
    assert!(r.drift() < 1e-3, "energy drifted by {}", r.drift());
}

#[test]
fn field_lines_leave_a_shell_from_its_surface_and_a_wire_all_round() {
    let scene = find("charged-shell").unwrap().build(&Params::default());
    let lines = scene.world.field_lines(12, Vec2::ZERO, 5.0, 0.0);
    // The bead's own lines are left out: these are the shell's twelve, starting on its surface.
    assert_eq!(lines.len(), 12);
    for line in &lines {
        assert!(relative(line[0].length(), 0.8) < 1e-9);
        assert!(line.last().unwrap().length() > 3.0);
    }
    let scene = find("charged-wire").unwrap().build(&Params::default());
    assert_eq!(scene.world.field_lines(12, Vec2::ZERO, 5.0, 0.0).len(), 12);
}

#[test]
fn lines_arriving_at_a_lone_negative_charge_come_in_from_far_away() {
    let mut world = World::new(Vec2::ZERO);
    world.add(Body::particle(1.0).charge(-1e-6));
    world.add_force(Electrostatics::default());
    let lines = world.field_lines(8, Vec2::ZERO, 2.0, 0.0);
    assert_eq!(lines.len(), 8);
    for line in &lines {
        assert!(line[0].length() > 2.0 && line.last().unwrap().length() < 1e-9);
    }
}

#[test]
fn distributions_and_a_surface_survive_a_round_trip_through_json() {
    for id in [
        "gauss-law",
        "charged-wire",
        "charged-sheet",
        "charged-shell",
    ] {
        let file = find(id).unwrap().scene_file(&Params::default());
        let back = SceneFile::from_json(&file.to_json()).unwrap();
        assert_eq!(file, back, "{id}");
    }
}
