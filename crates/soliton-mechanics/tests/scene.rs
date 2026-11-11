use soliton_mechanics::scenario::{catalogue, find, Params};
use soliton_mechanics::scene::SceneFile;
use soliton_mechanics::{Finished, Simulate, Simulation, Trigger};

#[test]
fn every_scenario_round_trips_through_json() {
    for scenario in catalogue() {
        let file = scenario.scene_file(&Params::default());
        let json = file.to_json();
        let back =
            SceneFile::from_json(&json).unwrap_or_else(|e| panic!("{}: {e}", scenario.spec.id));
        assert_eq!(back, file, "{}", scenario.spec.id);

        let mut a = Simulation::new(file.build().unwrap().world, file.world.dt);
        let mut b = Simulation::new(back.build().unwrap().world, back.world.dt);
        a.advance(300);
        b.advance(300);
        for id in a.world().bodies().ids() {
            assert_eq!(
                a.world().bodies().position(id),
                b.world().bodies().position(id),
                "{}",
                scenario.spec.id
            );
        }
    }
}

#[test]
fn a_hand_written_scene_builds_and_runs() {
    let json = r#"{
      "schema": 1,
      "title": "Block pulled by a hanging mass",
      "world": { "gravity": [0, -9.8], "dt": 0.001 },
      "bodies": [
        { "id": "table", "kind": "static", "shape": { "polygon": [[-3, -0.4], [3, -0.4], [3, 0], [-3, 0]] }, "friction": [0.2, 0.15] },
        { "id": "block", "kind": "rigid", "mass": 2, "shape": { "rectangle": [0.6, 0.4] }, "position": [0, 0.2] },
        { "id": "weight", "kind": "particle", "mass": 1, "shape": { "circle": 0.1 }, "position": [3.1, -1] }
      ],
      "constraints": [
        { "rope": { "nodes": [
          { "attach": { "body": "block", "local": [0.3, 0] } },
          { "pulley": { "center": [3, 0.1], "radius": 0.1, "wrap": "clockwise" } },
          { "attach": { "body": "weight" } }
        ] } }
      ],
      "focus": "block",
      "answers": [ { "name": "Acceleration (m/s²)", "value": 2.0 } ]
    }"#;
    let file = SceneFile::from_json(json).unwrap();
    let scene = file.build().unwrap();
    assert_eq!(scene.labels, ["table", "block", "weight"]);
    assert_eq!(scene.exact[0].1, 2.0);
    let mut sim = Simulation::new(scene.world, scene.dt);
    sim.advance(500);
    let v = sim.world().bodies().velocity(scene.focus).x;
    let expected = (1.0 - 0.15 * 2.0) / 3.0 * 9.8 * sim.time();
    assert!(
        (v - expected).abs() / expected < 1e-6,
        "v {v} expected {expected}"
    );
}

#[test]
fn scene_errors_name_the_problem() {
    let bad = SceneFile::from_json(
        r#"{ "schema": 7, "title": "x", "world": {"gravity": [0,0], "dt": 0.1}, "bodies": [] }"#,
    );
    assert!(bad.unwrap_err().to_string().contains("schema 7"));

    let mut file = find("atwood").unwrap().scene_file(&Params::default());
    file.focus = Some("nobody".into());
    assert!(file
        .build()
        .unwrap_err()
        .to_string()
        .contains("unknown body `nobody`"));

    let text = find("projectile")
        .unwrap()
        .scene_file(&Params::default())
        .to_json()
        .replace("\"mass\": 1.0", "\"mass\": -1.0");
    let built = SceneFile::from_json(&text).unwrap().build();
    assert!(built
        .unwrap_err()
        .to_string()
        .contains("mass must be positive"));
}

/// A ball dropped 5 m onto the ground, with the given finish rules.
fn dropped_ball(finish: &str) -> Simulation {
    let json = format!(
        r#"{{
      "schema": 1,
      "title": "Dropped ball",
      "world": {{ "gravity": [0, -9.8], "dt": 0.001 }},
      "bodies": [
        {{ "id": "ground", "kind": "static", "shape": {{ "halfplane": {{ "normal": [0, 1], "offset": 0 }} }}, "friction": [1, 1] }},
        {{ "id": "ball", "kind": "particle", "mass": 1, "shape": {{ "circle": 0.3 }}, "position": [0, 5.3], "restitution": 0.8 }}
      ],
      "finish": {finish}
    }}"#
    );
    let scene = SceneFile::from_json(&json).unwrap().build().unwrap();
    let mut sim = Simulation::new(scene.world, scene.dt);
    sim.finish_when(scene.finish);
    sim
}

/// Steps until the reported end changes and returns it with the time it was reported at.
fn run_to_finish(sim: &mut Simulation, limit: f64) -> Option<(Finished, f64)> {
    let before = sim.finished();
    while sim.time() < limit {
        sim.step();
        if sim.finished() != before {
            return sim.finished().map(|f| (f, sim.time()));
        }
    }
    None
}

fn labels() -> Vec<String> {
    vec!["ground".to_string(), "ball".to_string()]
}

#[test]
fn a_contact_rule_ends_the_problem_at_the_impact() {
    let landing = (2.0 * 5.0 / 9.8f64).sqrt();
    let mut sim = dropped_ball(r#"[ { "when": { "contact": ["ball", "ground"] } } ]"#);
    let (finished, reported) = run_to_finish(&mut sim, 3.0).unwrap();
    assert!((finished.time - landing).abs() < 1e-5, "{}", finished.time);
    assert!(reported - finished.time < 4e-3, "reported at {reported}");
    assert_eq!(finished.describe(&labels()), "ball touched ground");

    let mut held =
        dropped_ball(r#"[ { "when": { "contact": ["ground", "ball"] }, "hold": 0.3 } ]"#);
    let (finished, reported) = run_to_finish(&mut held, 3.0).unwrap();
    assert!((finished.time - landing - 0.3).abs() < 1e-5);
    assert!(reported >= finished.time && reported - finished.time < 2e-3);
}

#[test]
fn separation_time_and_rest_rules_fire() {
    let mut sim = dropped_ball(r#"[ { "when": { "separation": ["ball", "ground"] } } ]"#);
    let (finished, _) = run_to_finish(&mut sim, 3.0).unwrap();
    assert!((finished.time - 1.012).abs() < 2e-3, "{}", finished.time);
    assert_eq!(finished.describe(&labels()), "ball left ground");

    let mut sim = dropped_ball(r#"[ { "when": { "time": 0.5 } } ]"#);
    let (finished, reported) = run_to_finish(&mut sim, 3.0).unwrap();
    assert_eq!(finished.time, 0.5);
    assert!((reported - 0.502).abs() < 1e-9);
    assert_eq!(
        finished.describe(&labels()),
        "the problem's time window ended"
    );

    let mut sim = dropped_ball(r#"[ { "when": { "rest": 0.2 } } ]"#);
    let (finished, reported) = run_to_finish(&mut sim, 20.0).unwrap();
    assert!(finished.time > 5.0, "the ball bounces for a while first");
    assert!((reported - finished.time - 0.202).abs() < 1e-9);
    assert_eq!(finished.describe(&labels()), "everything came to rest");
}

#[test]
fn the_first_rule_to_fire_wins_and_rest_is_reported_afterwards() {
    let rules = r#"[ { "when": { "time": 4 } }, { "when": { "contact": ["ball", "ground"] } } ]"#;
    let mut sim = dropped_ball(rules);
    let (first, _) = run_to_finish(&mut sim, 20.0).unwrap();
    assert!(matches!(first.trigger, Trigger::Contact(..)));
    let (second, _) = run_to_finish(&mut sim, 20.0).unwrap();
    assert!(matches!(second.trigger, Trigger::Rest(_)));
    assert!(second.time > first.time);
    assert!(run_to_finish(&mut sim, 20.0).is_none());

    let mut unwatched = dropped_ball("[]");
    assert!(run_to_finish(&mut unwatched, 12.0).is_none());
}

#[test]
fn a_scene_at_rest_from_the_start_says_so() {
    let scene = find("pushed-block")
        .unwrap()
        .build(&Params::default().set("push", 8.0));
    let labels = scene.labels.clone();
    let mut sim = Simulation::new(scene.world, scene.dt);
    sim.finish_when(scene.finish);
    let (finished, reported) = run_to_finish(&mut sim, 3.0).unwrap();
    assert_eq!(finished.time, 0.0);
    assert!((reported - 0.502).abs() < 1e-9);
    assert_eq!(
        finished.describe(&labels),
        "nothing moves, it is in equilibrium from the start"
    );
}

#[test]
fn finish_rules_are_validated() {
    let unknown = SceneFile::from_json(
        r#"{ "schema": 1, "title": "x", "world": {"gravity": [0,0], "dt": 0.1},
             "bodies": [ { "id": "a", "kind": "particle", "mass": 1, "shape": { "circle": 0.1 } } ],
             "finish": [ { "when": { "contact": ["a", "nope"] } } ] }"#,
    )
    .unwrap()
    .build();
    assert!(unknown
        .unwrap_err()
        .to_string()
        .contains("unknown body `nope`"));

    let negative = SceneFile::from_json(
        r#"{ "schema": 1, "title": "x", "world": {"gravity": [0,0], "dt": 0.1},
             "bodies": [ { "id": "a", "kind": "particle", "mass": 1, "shape": { "circle": 0.1 } } ],
             "finish": [ { "when": { "time": 1 }, "hold": -1 } ] }"#,
    )
    .unwrap()
    .build();
    assert!(negative
        .unwrap_err()
        .to_string()
        .contains("finish.hold must not be negative"));
}
