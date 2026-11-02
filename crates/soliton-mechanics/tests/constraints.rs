use soliton_mechanics::exact;
use soliton_mechanics::{
    Anchor, Body, EventKind, ForceLabel, PinJoint, Polygon, Recorder, Rod, Rope, RopeNode,
    Simulate, Simulation, Slot, Vec2, World, Wrap,
};

const G: f64 = 9.8;

fn relative(measured: f64, expected: f64) -> f64 {
    (measured - expected).abs() / expected.abs().max(1e-300)
}

#[test]
fn atwood_machine_gives_textbook_acceleration_and_tension() {
    let (m1, m2, radius) = (2.0, 3.0, 0.1);
    let mut world = World::new(Vec2::new(0.0, -G));
    let left = world.add(Body::particle(m1).at(-radius, -1.0));
    let right = world.add(Body::particle(m2).at(radius, -1.0));
    let pulley = RopeNode::Pulley {
        center: Vec2::ZERO,
        radius,
        wrap: Wrap::Clockwise,
    };
    world.add_constraint(Rope::taut(
        [RopeNode::attach(left), pulley, RopeNode::attach(right)],
        world.bodies(),
    ));

    let mut sim = Simulation::new(world, 1e-3);
    let mut recorder = Recorder::every(1);
    sim.run_until(1.0, &mut recorder);

    let a = exact::atwood_acceleration(m1, m2, G);
    let t = sim.time();
    let bodies = sim.world().bodies();
    assert!(relative(bodies.velocity(left).y, a * t) < 1e-10);
    assert!(relative(bodies.velocity(right).y, -a * t) < 1e-10);
    assert!(relative(bodies.position(left).y, -1.0 + 0.5 * a * t * t) < 1e-10);
    assert!(
        (bodies.position(left).x + radius).abs() < 1e-12,
        "the rope must pull straight up"
    );

    let tension = recorder.force(left, ForceLabel::Tension);
    let expected = exact::atwood_tension(m1, m2, G);
    assert!(
        relative(tension.last().unwrap().y, expected) < 1e-10,
        "tension {:?}",
        tension.last()
    );
    assert!(tension.last().unwrap().x.abs() < 1e-12);

    let acceleration = recorder.body(left).acceleration.last().unwrap();
    assert!(relative(acceleration.y, a) < 1e-10);
    assert!(
        sim.events()
            .iter()
            .filter(|e| matches!(e.kind, EventKind::RopeSlack { .. }))
            .count()
            == 0
    );
}

#[test]
fn block_on_smooth_table_pulled_by_hanging_mass() {
    let (m1, m2, radius) = (4.0, 1.0, 0.05);
    let mut world = World::new(Vec2::new(0.0, -G));
    let block = world.add(Body::particle(m1).at(0.0, 0.0));
    let hanging = world.add(Body::particle(m2).at(2.0 + radius, -1.0));
    world.add_constraint(Slot::along(block, Vec2::ZERO, Vec2::X));
    let pulley = RopeNode::Pulley {
        center: Vec2::new(2.0, -radius),
        radius,
        wrap: Wrap::Clockwise,
    };
    world.add_constraint(Rope::taut(
        [RopeNode::attach(block), pulley, RopeNode::attach(hanging)],
        world.bodies(),
    ));

    let mut sim = Simulation::new(world, 1e-3);
    let mut recorder = Recorder::every(1);
    sim.run_until(0.5, &mut recorder);

    let (a, tension) = exact::table_and_hanging_mass(m1, m2, G);
    let bodies = sim.world().bodies();
    assert!(relative(bodies.velocity(block).x, a * sim.time()) < 1e-10);
    assert!(bodies.position(block).y.abs() < 1e-12);
    assert!(
        relative(
            recorder.force(block, ForceLabel::Tension).last().unwrap().x,
            tension
        ) < 1e-10
    );
    assert!(
        relative(
            recorder.force(block, ForceLabel::Normal).last().unwrap().y,
            m1 * G
        ) < 1e-10
    );
}

fn pendulum_period(amplitude: f64, dt: f64, periods: usize) -> (f64, f64) {
    let length = 1.0;
    let mut world = World::new(Vec2::new(0.0, -G));
    let bob =
        world.add(Body::particle(1.0).at(length * amplitude.sin(), -length * amplitude.cos()));
    world.add_constraint(Rod::new(
        Anchor::Fixed(Vec2::ZERO),
        Anchor::body(bob),
        length,
    ));
    let mut sim = Simulation::new(world, dt);
    let mut recorder = Recorder::every(1);
    let expected = exact::pendulum_period(length, G, amplitude);
    sim.run_until(periods as f64 * expected, &mut recorder);

    let x = &recorder.body(bob).position;
    let crossings: Vec<f64> = (1..x.len())
        .filter(|&i| x[i - 1].x > 0.0 && x[i].x <= 0.0)
        .map(|i| recorder.time[i - 1] + dt * x[i - 1].x / (x[i - 1].x - x[i].x))
        .collect();
    let measured = (crossings.last().unwrap() - crossings[0]) / (crossings.len() - 1) as f64;

    let e0 = recorder.energy[0].conserved();
    let drift = recorder
        .energy
        .iter()
        .map(|e| (e.conserved() - e0).abs())
        .fold(0.0, f64::max)
        / e0.abs();
    (measured, drift)
}

#[test]
fn small_pendulum_has_textbook_period() {
    let (measured, drift) = pendulum_period(0.01, 1e-3, 20);
    assert!(relative(measured, exact::pendulum_period_small(1.0, G)) < 1e-5);
    assert!(drift < 1e-6, "energy drift {drift:e}");
}

#[test]
fn large_pendulum_matches_elliptic_integral() {
    let amplitude = 2.0;
    let (measured, drift) = pendulum_period(amplitude, 1e-4, 10);
    assert!(relative(measured, exact::pendulum_period(1.0, G, amplitude)) < 1e-6);
    assert!(drift < 1e-6, "energy drift {drift:e}");

    let (coarse, _) = pendulum_period(amplitude, 2e-4, 10);
    let error_fine = relative(measured, exact::pendulum_period(1.0, G, amplitude));
    let error_coarse = relative(coarse, exact::pendulum_period(1.0, G, amplitude));
    let order = (error_coarse / error_fine).log2();
    assert!(
        order > 1.8,
        "period error should converge at second order, observed {order}"
    );
}

fn vertical_circle(speed_factor: f64) -> Vec<EventKind> {
    let length = 0.5;
    let mut world = World::new(Vec2::new(0.0, -G));
    let speed = speed_factor * exact::vertical_circle_min_speed(length, G);
    let bob = world.add(Body::particle(0.2).at(0.0, -length).velocity(speed, 0.0));
    world.add_constraint(Rope::new(
        [RopeNode::Peg(Vec2::ZERO), RopeNode::attach(bob)],
        length,
    ));
    let mut sim = Simulation::new(world, 1e-4);
    sim.advance(20000);
    sim.events().iter().map(|e| e.kind).collect()
}

#[test]
fn string_in_vertical_circle_goes_slack_only_below_critical_speed() {
    let fast = vertical_circle(1.05);
    assert!(
        !fast
            .iter()
            .any(|k| matches!(k, EventKind::RopeSlack { .. })),
        "{fast:?}"
    );

    let slow = vertical_circle(0.95);
    assert!(
        slow.iter()
            .any(|k| matches!(k, EventKind::RopeSlack { .. })),
        "{slow:?}"
    );
}

fn seesaw(m1: f64, d1: f64, m2: f64, d2: f64) -> (f64, f64) {
    let rod_mass = 1.0;
    let mut world = World::new(Vec2::new(0.0, -G));
    let rod = world.add(Body::rigid(rod_mass, Polygon::rectangle(4.0, 0.05)));
    let left = world.add(Body::particle(m1).at(-d1, 0.0));
    let right = world.add(Body::particle(m2).at(d2, 0.0));
    world.add_constraint(PinJoint::to_world(rod, Vec2::ZERO, Vec2::ZERO));
    world.add_constraint(PinJoint::between(
        left,
        Vec2::ZERO,
        rod,
        Vec2::new(-d1, 0.0),
    ));
    world.add_constraint(PinJoint::between(
        right,
        Vec2::ZERO,
        rod,
        Vec2::new(d2, 0.0),
    ));
    let inertia = world.bodies().inertia(rod);
    let dt = 1e-4;
    let mut sim = Simulation::new(world, dt);
    sim.step();
    (
        sim.world().bodies().angular_velocity(rod) / dt,
        exact::seesaw_angular_acceleration(m1, d1, m2, d2, inertia, G),
    )
}

#[test]
fn seesaw_balances_or_tips_as_the_torques_say() {
    let (alpha, _) = seesaw(2.0, 1.5, 3.0, 1.0);
    assert!(
        alpha.abs() < 1e-9,
        "balanced seesaw should not turn, alpha = {alpha:e}"
    );

    let (alpha, expected) = seesaw(2.0, 1.0, 3.0, 1.5);
    assert!(
        relative(alpha, expected) < 1e-6,
        "alpha {alpha} expected {expected}"
    );
}

#[test]
fn massive_pulley_shares_the_load_between_two_tensions() {
    let (m1, m2, pulley_mass, radius) = (2.0, 3.0, 4.0, 0.4);
    let mut world = World::new(Vec2::new(0.0, -G));
    let pulley = world.add(Body::rigid(
        pulley_mass,
        soliton_mechanics::Circle::new(radius),
    ));
    let left = world.add(Body::particle(m1).at(-radius, -3.0));
    let right = world.add(Body::particle(m2).at(radius, -3.0));
    world.add_constraint(PinJoint::to_world(pulley, Vec2::ZERO, Vec2::ZERO));
    let wheel = RopeNode::Wheel {
        body: pulley,
        radius,
        wrap: Wrap::Clockwise,
    };
    world.add_constraint(Rope::taut(
        [RopeNode::attach(left), wheel, RopeNode::attach(right)],
        world.bodies(),
    ));

    let mut sim = Simulation::new(world, 1e-3);
    let mut recorder = Recorder::every(1);
    sim.run_until(1.0, &mut recorder);

    let a = exact::atwood_massive_acceleration(m1, m2, pulley_mass, G);
    let t = sim.time();
    let bodies = sim.world().bodies();
    assert!(
        relative(bodies.velocity(right).y, -a * t) < 1e-8,
        "v = {}",
        bodies.velocity(right).y
    );
    assert!(
        relative(bodies.angular_velocity(pulley).abs() * radius, a * t) < 1e-8,
        "no slip on the rim"
    );
    let t_left = recorder
        .force(left, ForceLabel::Tension)
        .last()
        .unwrap()
        .length();
    let t_right = recorder
        .force(right, ForceLabel::Tension)
        .last()
        .unwrap()
        .length();
    assert!(
        relative(t_left, m1 * (G + a)) < 1e-8,
        "left tension {t_left}"
    );
    assert!(
        relative(t_right, m2 * (G - a)) < 1e-8,
        "right tension {t_right}"
    );
    assert!(
        t_left < t_right,
        "the heavier side must pull harder to spin the pulley"
    );
}

#[test]
fn flywheel_unwinds_at_the_textbook_rate() {
    let (wheel_mass, radius, m) = (10.0, 0.5, 2.0);
    let mut world = World::new(Vec2::new(0.0, -G));
    let wheel = world.add(Body::rigid(
        wheel_mass,
        soliton_mechanics::Circle::new(radius),
    ));
    let mass = world.add(Body::particle(m).at(radius, -2.0));
    world.add_constraint(PinJoint::to_world(wheel, Vec2::ZERO, Vec2::ZERO));
    let rim = RopeNode::Wheel {
        body: wheel,
        radius,
        wrap: Wrap::Clockwise,
    };
    world.add_constraint(Rope::taut([rim, RopeNode::attach(mass)], world.bodies()));

    let mut sim = Simulation::new(world, 1e-3);
    sim.advance(1000);
    let a = m * G / (m + 0.5 * wheel_mass);
    let bodies = sim.world().bodies();
    assert!(relative(-bodies.velocity(mass).y, a * sim.time()) < 1e-8);
    assert!(
        relative(
            bodies.angular_velocity(wheel).abs() * radius,
            a * sim.time()
        ) < 1e-8
    );
}
