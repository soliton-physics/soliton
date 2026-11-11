//! The hand on a body and scene snapshots (design 008).

use soliton_mechanics::scenario::{catalogue, find, Params};
use soliton_mechanics::scene::SceneFile;
use soliton_mechanics::{
    Body, Circle, ForceLabel, HalfPlane, Polygon, Recorder, Simulate, Simulation, Vec2, World,
};

const G: f64 = 9.8;

fn ball_in_the_air() -> (World, soliton_mechanics::BodyId) {
    let mut world = World::new(Vec2::new(0.0, -G));
    let ball = world.add(Body::particle(2.0).shape(Circle::new(0.2)).at(0.0, 1.0));
    (world, ball)
}

#[test]
fn a_held_body_settles_under_the_hand_carrying_its_weight() {
    let (mut world, ball) = ball_in_the_air();
    assert!(world.grab(ball, Vec2::new(0.0, 1.0), Vec2::new(1.0, 2.0)));
    let mut sim = Simulation::new(world, 1e-3);
    let mut recorder = Recorder::every(1);
    sim.run_until(2.0, &mut recorder);

    let bodies = sim.world().bodies();
    assert!(
        bodies.velocity(ball).length() < 1e-9,
        "{:?}",
        bodies.velocity(ball)
    );
    // The spring sags by m g / k = g / ω² below the hand.
    let sag = G / (30.0 * 30.0);
    let hang = bodies.position(ball) - Vec2::new(1.0, 2.0 - sag);
    assert!(hang.length() < 1e-9, "{hang:?}");
    let hand = *recorder.force(ball, ForceLabel::Applied).last().unwrap();
    assert!((hand - Vec2::new(0.0, 2.0 * G)).length() < 1e-8, "{hand:?}");
}

#[test]
fn the_hand_does_external_work_and_the_ledger_balances() {
    let (mut world, ball) = ball_in_the_air();
    world.grab(ball, Vec2::new(0.0, 1.0), Vec2::new(0.0, 1.0));
    let mut sim = Simulation::new(world, 1e-3);
    let start = sim.energy().conserved();
    for step in 0..3000 {
        let t = step as f64 * 1e-3;
        sim.world_mut()
            .move_grab(Vec2::new((2.0 * t).sin(), 1.0 + 0.5 * t));
        sim.step();
    }
    let energy = sim.energy();
    assert!(energy.external_work.abs() > 1.0, "the hand lifted the ball");
    // Second order in `ω dt` with the spring's ω = 30 rad/s: (0.03)² is about 1e-3.
    let drift = (energy.conserved() - start).abs() / energy.external_work.abs();
    assert!(drift < 1e-3, "drift {drift}");

    sim.world_mut().release();
    let held = sim.energy().external_work;
    sim.advance(500);
    assert_eq!(
        sim.energy().external_work,
        held,
        "nothing works on a released body"
    );
    let a = sim.world().acceleration(ball);
    assert!((a - Vec2::new(0.0, -G)).length() < 1e-12, "{a:?}");
}

#[test]
fn a_thrown_body_keeps_the_velocity_the_hand_gave_it() {
    let (mut world, ball) = ball_in_the_air();
    world.grab(ball, Vec2::new(0.0, 1.0), Vec2::new(0.0, 1.0));
    let mut sim = Simulation::new(world, 1e-3);
    for step in 0..1500 {
        sim.world_mut()
            .move_grab(Vec2::new(3.0 * step as f64 * 1e-3, 1.0));
        sim.step();
    }
    sim.world_mut().release();
    let vx = sim.world().bodies().velocity(ball).x;
    assert!(
        (vx - 3.0).abs() < 0.05,
        "followed the hand at 3 m/s, got {vx}"
    );
    sim.advance(200);
    assert_eq!(sim.world().bodies().velocity(ball).x, vx);
}

#[test]
fn static_bodies_cannot_be_held_and_picking_respects_shapes() {
    let mut world = World::new(Vec2::new(0.0, -G));
    let ground = world.add(Body::fixed(HalfPlane::new(Vec2::Y, 0.0)));
    let crate_ = world.add(
        Body::rigid(1.0, Polygon::rectangle(1.0, 0.4))
            .at(0.0, 0.2)
            .angle(0.0),
    );
    let speck = world.add(Body::particle(1.0).at(3.0, 2.0));
    assert!(!world.grab(ground, Vec2::ZERO, Vec2::ZERO));
    assert!(world.grabbed().is_none());

    assert_eq!(world.body_at(Vec2::new(0.45, 0.35), 0.0), Some(crate_));
    assert_eq!(world.body_at(Vec2::new(0.45, 0.45), 0.0), None);
    assert_eq!(world.body_at(Vec2::new(3.1, 2.0), 0.0), None);
    assert_eq!(world.body_at(Vec2::new(3.1, 2.0), 0.2), Some(speck));
    assert_eq!(world.body_at(Vec2::new(0.0, -5.0), 0.2), None);
}

#[test]
fn a_snapshot_carries_on_from_where_the_run_was() {
    for scenario in catalogue() {
        let file = scenario.scene_file(&Params::default());
        let mut original = Simulation::new(file.build().unwrap().world, file.world.dt);
        original.advance(400);
        let snapshot = file.snapshot(original.world(), original.time()).unwrap();
        let reread = SceneFile::from_json(&snapshot.to_json()).unwrap();
        assert_eq!(reread, snapshot, "{}", scenario.spec.id);

        let mut resumed = Simulation::new(reread.build().unwrap().world, reread.world.dt);
        original.advance(200);
        resumed.advance(200);
        let (a, b) = (original.world().bodies(), resumed.world().bodies());
        for id in a.ids() {
            let gap = (a.position(id) - b.position(id)).length();
            assert!(gap < 1e-6, "{}: body {id:?} off by {gap}", scenario.spec.id);
        }
    }
}

#[test]
fn a_snapshot_shifts_time_windows_and_drops_the_ones_that_passed() {
    let json = r#"{
      "schema": 1, "title": "Pushed for a while", "world": { "gravity": [0, 0], "dt": 0.001 },
      "bodies": [ { "id": "puck", "kind": "particle", "mass": 1, "shape": { "circle": 0.1 } } ],
      "forces": [
        { "applied": { "body": "puck", "force": [1, 0], "until": 0.2 } },
        { "applied": { "body": "puck", "force": [0, 1], "from": 0.1, "until": 2 } }
      ],
      "finish": [ { "when": { "time": 0.3 } }, { "when": { "time": 3 } } ]
    }"#;
    let file = SceneFile::from_json(json).unwrap();
    let mut sim = Simulation::new(file.build().unwrap().world, 1e-3);
    sim.advance(500);
    let snapshot = file.snapshot(sim.world(), sim.time()).unwrap();
    let text = snapshot.to_json();
    assert_eq!(snapshot.forces.len(), 1, "{text}");
    assert!(
        text.contains("\"until\": 1.5") && !text.contains("\"from\""),
        "{text}"
    );
    assert_eq!(snapshot.finish.len(), 1);
    assert!(text.contains("\"time\": 2.5"), "{text}");
}

#[test]
fn polygons_land_where_they_are_drawn_in_either_winding() {
    let scene = |points: &str| {
        let json = format!(
            r#"{{ "schema": 1, "title": "x", "world": {{ "gravity": [0, -9.8], "dt": 0.001 }},
                 "bodies": [ {{ "id": "wedge", "kind": "static", "shape": {{ "polygon": {points} }} }},
                             {{ "id": "ball", "kind": "particle", "mass": 1, "shape": {{ "circle": 0.1 }}, "position": [9, 9] }} ] }}"#
        );
        SceneFile::from_json(&json).unwrap().build().unwrap()
    };
    let ccw = scene("[[0,0],[4,0],[0,2]]");
    let cw = scene("[[0,0],[0,2],[4,0]]");
    let id = soliton_mechanics::BodyId::from_index(0);
    let (a, b) = (
        ccw.world.bodies().position(id),
        cw.world.bodies().position(id),
    );
    assert!(
        (a - Vec2::new(4.0 / 3.0, 2.0 / 3.0)).length() < 1e-12,
        "{a:?}"
    );
    assert!((a - b).length() < 1e-12, "{b:?}");
    let _ = find("atwood");
}
