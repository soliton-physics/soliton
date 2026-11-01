use soliton_mechanics::scenario::{catalogue, find, Params};
use soliton_mechanics::scene::SceneFile;
use soliton_mechanics::{Simulate, Simulation};

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
