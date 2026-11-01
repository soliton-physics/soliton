use soliton_mechanics::exact;
use soliton_mechanics::{
    Anchor, Applied, Body, Circle, ForceLabel, LinearDrag, Recorder, Simulate, Simulation, Spring,
    Torque, Vec2, World,
};

const G: f64 = 9.8;

#[test]
fn projectile_is_exact_at_any_step() {
    let (speed, angle): (f64, f64) = (20.0, 0.6);
    let mut world = World::new(Vec2::new(0.0, -G));
    let ball = world.add(Body::particle(0.5).velocity(speed * angle.cos(), speed * angle.sin()));
    let flight = exact::projectile_flight_time(speed, angle, G);
    let steps = 200;
    let mut sim = Simulation::new(world, flight / steps as f64);
    sim.advance(steps);

    let position = sim.world().bodies().position(ball);
    let expected = exact::constant_acceleration(
        Vec2::ZERO,
        Vec2::from_polar(speed, angle),
        Vec2::new(0.0, -G),
        steps as f64 * sim.dt(),
    );
    assert!((position - expected).length() < 1e-10);
    assert!((position.x - exact::projectile_range(speed, angle, G)).abs() < 1e-9);
    assert!(
        position.y.abs() < 1e-9,
        "should be back on the ground, y = {}",
        position.y
    );
}

#[test]
fn spring_period_matches_verlet_dispersion_and_textbook() {
    let (mass, stiffness, amplitude, dt) = (1.0, 4.0 * std::f64::consts::PI.powi(2), 0.1, 1e-3);
    let mut world = World::new(Vec2::ZERO);
    let block = world.add(Body::particle(mass).at(amplitude, 0.0));
    world.add_force(Spring::new(
        Anchor::Fixed(Vec2::ZERO),
        Anchor::body(block),
        stiffness,
        0.0,
    ));
    let mut sim = Simulation::new(world, dt);
    let mut recorder = Recorder::every(1);
    sim.run_until(100.0, &mut recorder);

    let x = &recorder.body(block).position;
    let crossings: Vec<f64> = (1..x.len())
        .filter(|&i| x[i - 1].x > 0.0 && x[i].x <= 0.0)
        .map(|i| recorder.time[i - 1] + dt * x[i - 1].x / (x[i - 1].x - x[i].x))
        .collect();
    let measured = (crossings.last().unwrap() - crossings[0]) / (crossings.len() - 1) as f64;

    let omega = (stiffness / mass).sqrt();
    let verlet = exact::verlet_period(omega, dt);
    let textbook = exact::spring_period(mass, stiffness);
    assert!(
        (measured - verlet).abs() / verlet < 1e-8,
        "measured {measured}, verlet {verlet}"
    );
    assert!((measured - textbook).abs() / textbook < (omega * dt).powi(2) / 12.0);

    let e0 = recorder.energy[0].conserved();
    let drift = recorder
        .energy
        .iter()
        .map(|e| (e.conserved() - e0).abs())
        .fold(0.0, f64::max);
    assert!(drift / e0 < 1e-5, "energy drift {drift:e}");
}

fn damped_error(dt: f64) -> f64 {
    let (mass, stiffness, damping, x0) = (1.0, 10.0, 0.6, 0.2);
    let mut world = World::new(Vec2::ZERO);
    let block = world.add(Body::particle(mass).at(x0, 0.0));
    world.add_force(Spring::new(
        Anchor::Fixed(Vec2::ZERO),
        Anchor::body(block),
        stiffness,
        0.0,
    ));
    world.add_force(LinearDrag::new(block, damping));
    let mut sim = Simulation::new(world, dt);
    sim.advance((5.0 / dt).round() as usize);
    let exact = exact::damped_oscillator(x0, mass, stiffness, damping, sim.time());
    (sim.world().bodies().position(block).x - exact).abs()
}

#[test]
fn damped_oscillator_converges_at_second_order() {
    let coarse = damped_error(2e-3);
    let fine = damped_error(1e-3);
    let order = (coarse / fine).log2();
    assert!((order - 2.0).abs() < 0.1, "observed order {order}");
}

#[test]
fn drag_ledger_balances_energy() {
    let mut world = World::new(Vec2::ZERO);
    let block = world.add(Body::particle(2.0).velocity(3.0, 0.0));
    world.add_force(LinearDrag::new(block, 0.5));
    let mut sim = Simulation::new(world, 1e-3);
    let e0 = sim.energy().conserved();
    sim.advance(5000);
    let e = sim.energy();
    assert!(
        e.dissipated > 0.5 * e0,
        "most energy should be dissipated by now"
    );
    assert!(
        (e.conserved() - e0).abs() / e0 < 1e-7,
        "ledger drift {:e}",
        (e.conserved() - e0) / e0
    );
}

#[test]
fn applied_force_work_is_exact() {
    let mut world = World::new(Vec2::ZERO);
    let block = world.add(Body::particle(1.5));
    world.add_force(Applied::new(block, Vec2::new(3.0, 0.0)).until(2.0));
    let mut sim = Simulation::new(world, 1e-3);
    sim.advance(4000);
    let e = sim.energy();
    let kinetic = 0.5 * 1.5 * (3.0 / 1.5 * 2.0_f64).powi(2);
    assert!((e.kinetic - kinetic).abs() < 1e-9);
    assert!((e.external_work - kinetic).abs() < 1e-9);
    assert!(e.conserved().abs() < 1e-9);
}

#[test]
fn torque_spins_rigid_body_exactly() {
    let mut world = World::new(Vec2::ZERO);
    let disc = world.add(Body::rigid(2.0, Circle::new(0.5)));
    let inertia = world.bodies().inertia(disc);
    world.add_force(Torque::new(disc, 0.7));
    let mut sim = Simulation::new(world, 1e-3);
    sim.advance(3000);
    let t = sim.time();
    let bodies = sim.world().bodies();
    assert!((inertia - 0.25).abs() < 1e-15);
    assert!((bodies.angular_velocity(disc) - 0.7 * t / inertia).abs() < 1e-10);
    assert!((bodies.angle(disc) - 0.5 * 0.7 / inertia * t * t).abs() < 1e-9);
}

#[test]
fn free_body_diagram_reports_every_force() {
    let mut world = World::new(Vec2::new(0.0, -G));
    let block = world.add(Body::particle(2.0));
    world.add_force(Applied::new(block, Vec2::new(4.0, 0.0)));
    let mut sim = Simulation::new(world, 1e-3);
    let mut recorder = Recorder::every(1);
    sim.run_until(0.01, &mut recorder);

    let gravity = recorder.force(block, ForceLabel::Gravity);
    let applied = recorder.force(block, ForceLabel::Applied);
    assert_eq!(gravity[0], Vec2::new(0.0, -2.0 * G));
    assert_eq!(applied[0], Vec2::new(4.0, 0.0));
    let labels: Vec<_> = sim.world().forces_on(block).map(|s| s.label).collect();
    assert_eq!(labels, [ForceLabel::Gravity, ForceLabel::Applied]);
}
