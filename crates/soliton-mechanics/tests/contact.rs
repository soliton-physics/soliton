use soliton_mechanics::exact;
use soliton_mechanics::{
    Applied, Body, Circle, EventKind, ForceLabel, HalfPlane, Polygon, Recorder, Simulate,
    Simulation, Vec2, World,
};

const G: f64 = 9.8;

fn relative(measured: f64, expected: f64) -> f64 {
    (measured - expected).abs() / expected.abs().max(1e-300)
}

fn count(sim: &Simulation, pick: impl Fn(&EventKind) -> bool) -> usize {
    sim.events().iter().filter(|e| pick(&e.kind)).count()
}

/// A particle block resting on an incline through the origin, surface normal tilted by `theta`.
fn block_on_incline(
    theta: f64,
    static_mu: f64,
    kinetic_mu: f64,
) -> (World, soliton_mechanics::BodyId) {
    let mut world = World::new(Vec2::new(0.0, -G));
    let normal = Vec2::new(-theta.sin(), theta.cos());
    world.add(Body::fixed(HalfPlane::new(normal, 0.0)).friction(static_mu, kinetic_mu));
    let block = world.add(Body::particle(2.0).at(0.0, 0.0));
    (world, block)
}

#[test]
fn block_on_incline_rests_when_static_friction_suffices() {
    let theta: f64 = 0.35;
    let (world, block) = block_on_incline(theta, 0.5, 0.4);
    assert!(theta.tan() < 0.5);
    let mut sim = Simulation::new(world, 1e-3);
    let mut recorder = Recorder::every(1);
    sim.run_until(1.0, &mut recorder);

    let bodies = sim.world().bodies();
    assert!(
        bodies.velocity(block).length() < 1e-12,
        "{:?}",
        bodies.velocity(block)
    );
    assert!(bodies.position(block).length() < 1e-10);
    let friction = recorder
        .force(block, ForceLabel::Friction)
        .last()
        .unwrap()
        .length();
    let normal = recorder
        .force(block, ForceLabel::Normal)
        .last()
        .unwrap()
        .length();
    assert!(
        relative(friction, 2.0 * G * theta.sin()) < 1e-10,
        "friction {friction}"
    );
    assert!(
        relative(normal, 2.0 * G * theta.cos()) < 1e-10,
        "normal {normal}"
    );
    assert_eq!(
        count(&sim, |k| matches!(k, EventKind::FrictionSlip { .. })),
        0
    );
    assert_eq!(
        count(&sim, |k| matches!(k, EventKind::ContactBegin { .. })),
        1
    );
}

#[test]
fn block_on_incline_slides_with_kinetic_friction() {
    let theta: f64 = 0.5;
    let (world, block) = block_on_incline(theta, 0.2, 0.15);
    assert!(theta.tan() > 0.2);
    let mut sim = Simulation::new(world, 1e-3);
    sim.advance(1000);

    let a = exact::incline_sliding_acceleration(theta, 0.15, G);
    let speed = sim.world().bodies().velocity(block).length();
    assert!(
        relative(speed, a * sim.time()) < 1e-10,
        "speed {speed}, expected {}",
        a * sim.time()
    );
    assert_eq!(
        count(&sim, |k| matches!(k, EventKind::FrictionSlip { .. })),
        1
    );
}

#[test]
fn pushed_block_moves_only_past_static_friction() {
    let (mass, static_mu, kinetic_mu) = (2.0, 0.5, 0.3);
    let run = |push: f64| {
        let mut world = World::new(Vec2::new(0.0, -G));
        world.add(Body::fixed(HalfPlane::new(Vec2::Y, 0.0)).friction(static_mu, kinetic_mu));
        let block = world.add(Body::particle(mass));
        world.add_force(Applied::new(block, Vec2::new(push, 0.0)));
        let mut sim = Simulation::new(world, 1e-3);
        let mut recorder = Recorder::every(1);
        sim.run_until(1.0, &mut recorder);
        let friction = recorder
            .force(block, ForceLabel::Friction)
            .last()
            .unwrap()
            .x;
        (sim.world().bodies().velocity(block).x, friction, sim.time())
    };

    let (v, friction, _) = run(0.9 * static_mu * mass * G);
    assert!(v.abs() < 1e-12, "should not move, v = {v}");
    assert!(
        relative(friction, -0.9 * static_mu * mass * G) < 1e-10,
        "static friction balances the push"
    );

    let push = 1.1 * static_mu * mass * G;
    let (v, friction, t) = run(push);
    let a = (push - kinetic_mu * mass * G) / mass;
    assert!(relative(v, a * t) < 1e-10);
    assert!(relative(friction, -kinetic_mu * mass * G) < 1e-10);
}

#[test]
fn stopping_distance_and_work_energy_theorem() {
    let (mass, u, mu) = (1.5, 3.0, 0.2);
    let mut world = World::new(Vec2::new(0.0, -G));
    world.add(Body::fixed(HalfPlane::new(Vec2::Y, 0.0)).friction(mu, mu));
    let block = world.add(Body::particle(mass).velocity(u, 0.0));
    let dt = 1e-4;
    let mut sim = Simulation::new(world, dt);
    let e0 = sim.energy().conserved();
    sim.advance((2.0 * u / (mu * G) / dt) as usize);

    let bodies = sim.world().bodies();
    assert!(
        bodies.velocity(block).length() < 1e-12,
        "block should have stopped"
    );
    let distance = bodies.position(block).x;
    let expected = exact::stopping_distance(u, mu, G);
    assert!(
        relative(distance, expected) < 1e-8,
        "distance {distance} vs {expected}"
    );
    let ledger = sim.energy();
    assert!(
        relative(ledger.dissipated, 0.5 * mass * u * u) < 1e-8,
        "friction work {}",
        ledger.dissipated
    );
    assert!((ledger.conserved() - e0).abs() < 1e-6);
    assert_eq!(
        count(&sim, |k| matches!(k, EventKind::FrictionStick { .. })),
        1
    );
}

fn head_on(
    e: f64,
) -> (
    Simulation,
    soliton_mechanics::BodyId,
    soliton_mechanics::BodyId,
    f64,
) {
    let (m1, u1, m2, u2) = (1.0, 2.0, 3.0, 0.0);
    let mut world = World::new(Vec2::ZERO);
    let a = world.add(
        Body::particle(m1)
            .shape(Circle::new(0.1))
            .at(-1.0, 0.0)
            .velocity(u1, 0.0)
            .restitution(e),
    );
    let b = world.add(
        Body::particle(m2)
            .shape(Circle::new(0.1))
            .at(0.0, 0.0)
            .velocity(u2, 0.0)
            .restitution(e),
    );
    let mut sim = Simulation::new(world, 1e-3);
    let e0 = sim.energy().conserved();
    sim.advance(1000);
    (sim, a, b, e0)
}

#[test]
fn elastic_head_on_collision() {
    let (sim, a, b, _) = head_on(1.0);
    let (v1, v2) = exact::collision_1d(1.0, 2.0, 3.0, 0.0, 1.0);
    let bodies = sim.world().bodies();
    assert!(
        relative(bodies.velocity(a).x, v1) < 1e-10,
        "{}",
        bodies.velocity(a).x
    );
    assert!(
        relative(bodies.velocity(b).x, v2) < 1e-10,
        "{}",
        bodies.velocity(b).x
    );
    assert!(sim.energy().dissipated.abs() < 1e-12);
    assert_eq!(
        count(&sim, |k| matches!(k, EventKind::ContactBegin { .. })),
        1
    );
    assert_eq!(
        count(&sim, |k| matches!(k, EventKind::ContactEnd { .. })),
        1
    );
}

#[test]
fn perfectly_inelastic_collision_loses_the_textbook_energy() {
    let (sim, a, b, e0) = head_on(0.0);
    let (v1, v2) = exact::collision_1d(1.0, 2.0, 3.0, 0.0, 0.0);
    let bodies = sim.world().bodies();
    assert!(relative(bodies.velocity(a).x, v1) < 1e-10);
    assert!(relative(bodies.velocity(b).x, v2) < 1e-10);
    let lost = 0.5 * (1.0 * 3.0 / 4.0) * 2.0 * 2.0;
    let ledger = sim.energy();
    assert!(
        relative(ledger.dissipated, lost) < 1e-10,
        "dissipated {}",
        ledger.dissipated
    );
    assert!((ledger.conserved() - e0).abs() < 1e-10);
}

#[test]
fn oblique_elastic_collision_of_equal_discs_scatters_at_right_angles() {
    let mut world = World::new(Vec2::ZERO);
    let a = world.add(
        Body::particle(1.0)
            .shape(Circle::new(0.25))
            .at(-2.0, 0.0)
            .velocity(3.0, 0.0),
    );
    let b = world.add(Body::particle(1.0).shape(Circle::new(0.25)).at(0.0, 0.3));
    let mut sim = Simulation::new(world, 1e-4);
    let p0 = sim.momentum();
    let e0 = sim.energy().conserved();
    sim.advance(20000);

    let bodies = sim.world().bodies();
    let (va, vb) = (bodies.velocity(a), bodies.velocity(b));
    assert!((sim.momentum() - p0).length() < 1e-12);
    assert!((sim.energy().conserved() - e0).abs() < 1e-10);
    assert!(
        va.dot(vb).abs() < 1e-10,
        "equal masses scatter at 90°, got {va:?} {vb:?}"
    );
    assert!(vb.length() > 0.5, "target disc should have been struck");
}

fn rolling(static_mu: f64) -> (f64, f64, f64, usize) {
    let theta: f64 = 0.5;
    let radius = 0.2;
    let mut world = World::new(Vec2::new(0.0, -G));
    let normal = Vec2::new(-theta.sin(), theta.cos());
    world.add(Body::fixed(HalfPlane::new(normal, 0.0)).friction(static_mu, 0.9 * static_mu));
    let disc =
        world.add(Body::rigid(1.0, Circle::new(radius)).at(normal.x * radius, normal.y * radius));
    let mut sim = Simulation::new(world, 1e-3);
    sim.advance(1000);
    let bodies = sim.world().bodies();
    let slips = count(&sim, |k| matches!(k, EventKind::FrictionSlip { .. }));
    (
        bodies.velocity(disc).length(),
        bodies.angular_velocity(disc).abs() * radius,
        sim.time(),
        slips,
    )
}

#[test]
fn disc_rolls_without_slipping_when_friction_allows() {
    let theta: f64 = 0.5;
    let needed = exact::rolling_min_friction(theta, 0.5);
    let (speed, rim_speed, t, slips) = rolling(1.5 * needed);
    assert!(
        relative(speed, exact::rolling_acceleration(theta, 0.5, G) * t) < 1e-8,
        "speed {speed}"
    );
    assert!(relative(rim_speed, speed) < 1e-8, "no slip means v = ω R");
    assert_eq!(slips, 0);
}

#[test]
fn disc_slips_when_friction_is_too_small() {
    let theta: f64 = 0.5;
    let needed = exact::rolling_min_friction(theta, 0.5);
    let mu = 0.5 * needed;
    let (speed, rim_speed, t, slips) = rolling(mu);
    assert!(
        relative(
            speed,
            exact::incline_sliding_acceleration(theta, 0.9 * mu, G) * t
        ) < 1e-8
    );
    assert!(
        rim_speed < 0.9 * speed,
        "rim should lag the centre when slipping"
    );
    assert_eq!(slips, 1);
}

#[test]
fn bouncing_ball_heights_follow_restitution_squared() {
    let e = 0.8;
    let dt = 1e-4;
    let mut world = World::new(Vec2::new(0.0, -G));
    world.add(Body::fixed(HalfPlane::new(Vec2::Y, 0.0)));
    let ball = world.add(
        Body::particle(0.1)
            .shape(Circle::new(0.05))
            .at(0.0, 1.05)
            .restitution(e),
    );
    let mut sim = Simulation::new(world, dt);
    let mut recorder = Recorder::every(1);
    sim.run_until(2.5, &mut recorder);

    let y: Vec<f64> = recorder
        .body(ball)
        .position
        .iter()
        .map(|p| p.y - 0.05)
        .collect();
    let mut apexes = vec![1.0];
    for i in 1..y.len() - 1 {
        if y[i] > y[i - 1] && y[i] >= y[i + 1] && y[i] > 0.01 {
            apexes.push(y[i]);
        }
    }
    assert!(apexes.len() >= 4, "{apexes:?}");
    for pair in apexes.windows(2).take(3) {
        let ratio = pair[1] / pair[0];
        assert!(
            relative(ratio, e * e) < 5e-5,
            "apex ratio {ratio} vs {}",
            e * e
        );
    }
    let landing = sim
        .events()
        .iter()
        .find(|ev| matches!(ev.kind, EventKind::ContactBegin { .. }))
        .unwrap()
        .time;
    let expected = (2.0 / G).sqrt();
    assert!(
        (landing - expected).abs() < 1e-6,
        "first landing at {landing} vs {expected}"
    );
    let bounces = count(&sim, |k| matches!(k, EventKind::ContactBegin { .. }));
    assert!(bounces >= 4);
}

fn ladder(mu: f64) -> f64 {
    let theta: f64 = 1.0;
    let length = 4.0;
    let mut world = World::new(Vec2::new(0.0, -G));
    world.add(Body::fixed(HalfPlane::new(Vec2::Y, 0.0)).friction(mu, 0.9 * mu));
    world.add(Body::fixed(HalfPlane::new(Vec2::X, 0.0)));
    let ladder = world.add(
        Body::rigid(5.0, Polygon::rectangle(length, 0.02))
            .at(0.5 * length * theta.cos(), 0.5 * length * theta.sin())
            .angle(std::f64::consts::PI - theta),
    );
    let mut sim = Simulation::new(world, 1e-4);
    sim.advance(5000);
    sim.world().bodies().velocity(ladder).length()
}

#[test]
fn ladder_against_a_smooth_wall_needs_the_textbook_floor_friction() {
    let needed = exact::ladder_min_friction(1.0);
    assert!(
        ladder(1.3 * needed) < 1e-9,
        "enough friction: the ladder stays"
    );
    assert!(
        ladder(0.7 * needed) > 1e-3,
        "too little friction: the ladder slides"
    );
}

#[test]
fn box_rests_flat_on_the_floor() {
    let mut world = World::new(Vec2::new(0.0, -G));
    world.add(Body::fixed(HalfPlane::new(Vec2::Y, 0.0)).friction(0.5, 0.4));
    let block = world.add(Body::rigid(3.0, Polygon::rectangle(0.4, 0.2)).at(0.0, 0.1));
    let mut sim = Simulation::new(world, 1e-3);
    let mut recorder = Recorder::every(1);
    sim.run_until(1.0, &mut recorder);
    let bodies = sim.world().bodies();
    assert!(bodies.velocity(block).length() < 1e-12);
    assert!(bodies.angular_velocity(block).abs() < 1e-12);
    assert!(bodies.angle(block).abs() < 1e-12);
    assert!((bodies.position(block).y - 0.1).abs() < 1e-10);
    assert!(
        relative(
            recorder.force(block, ForceLabel::Normal).last().unwrap().y,
            3.0 * G
        ) < 1e-10
    );
}

/// A push below `μs N` applied at the block's side tilts the load onto the leading corner. That corner reaches its
/// own static limit first and the other takes the remainder: the block as a whole still holds.
#[test]
fn static_friction_is_shared_between_the_corners_of_a_pushed_box() {
    let (mass, push, mu_s, mu_k) = (2.0, 8.0, 0.5, 0.3);
    assert!(push < mu_s * mass * G && push > mu_k * mass * G);
    let mut world = World::new(Vec2::new(0.0, -G));
    world.add(Body::fixed(HalfPlane::new(Vec2::Y, 0.0)).friction(mu_s, mu_k));
    let block = world.add(Body::rigid(mass, Polygon::rectangle(0.8, 0.5)).at(0.0, 0.25));
    world.add_force(Applied::new(block, Vec2::new(push, 0.0)).at_local(Vec2::new(-0.4, 0.0)));
    let mut sim = Simulation::new(world, 1e-3);
    let mut recorder = Recorder::every(1);
    sim.run_until(1.0, &mut recorder);

    let bodies = sim.world().bodies();
    assert!(bodies.velocity(block).length() < 1e-10);
    assert!(bodies.position(block).x.abs() < 1e-10);
    for sample in [0, recorder.samples() - 1] {
        let friction = recorder.force(block, ForceLabel::Friction)[sample];
        let normal = recorder.force(block, ForceLabel::Normal)[sample];
        assert!(relative(-friction.x, push) < 1e-8, "friction {friction:?}");
        assert!(relative(normal.y, mass * G) < 1e-8, "normal {normal:?}");
    }
    assert_eq!(
        count(&sim, |k| matches!(k, EventKind::FrictionSlip { .. })),
        0
    );
}

#[test]
fn a_frictionless_surface_reports_no_friction_events() {
    let mut world = World::new(Vec2::new(0.0, -G));
    world.add(Body::fixed(HalfPlane::new(Vec2::Y, 0.0)));
    world.add(
        Body::particle(1.0)
            .shape(Circle::new(0.2))
            .at(0.0, 0.2)
            .velocity(2.0, 0.0),
    );
    let mut sim = Simulation::new(world, 1e-3);
    sim.advance(200);
    assert_eq!(
        count(&sim, |k| matches!(
            k,
            EventKind::FrictionSlip { .. } | EventKind::FrictionStick { .. }
        )),
        0
    );
}
