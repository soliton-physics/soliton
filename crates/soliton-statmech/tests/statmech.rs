//! Statistical mechanics (design 016): exact results the models must reproduce, within what a finite random
//! sample can give.

use soliton_statmech::scenario::{catalogue, find, Params};
use soliton_statmech::scene::{Run, StatScene};
use soliton_statmech::{
    onsager_energy, onsager_magnetisation, Darts, Ising, Walkers, CRITICAL_TEMPERATURE,
};

fn run(id: &str, params: &Params, steps: usize) -> Run {
    let mut run = find(id).unwrap().scene(params).start().unwrap();
    run.advance(steps);
    run
}

fn agree(run: &Run, id: &str) {
    let mut checked = 0;
    for (name, value, measured, within) in run.answers() {
        let Some(found) = measured else { continue };
        checked += 1;
        assert!(
            (found - value).abs() <= within,
            "{id}, {name}: {found} against {value} ± {within}"
        );
    }
    assert!(checked > 0, "{id}: nothing was measured");
}

#[test]
fn onsagers_formulas_have_their_known_values() {
    assert!((CRITICAL_TEMPERATURE - 2.0 / (1.0 + 2f64.sqrt()).ln()).abs() < 1e-14);
    assert_eq!(onsager_magnetisation(3.0), 0.0);
    assert!((onsager_magnetisation(1.0) - 0.999_275_8).abs() < 1e-6);
    // All spins aligned at absolute zero; −√2 at the critical point.
    assert!((onsager_energy(1e-3) + 2.0).abs() < 1e-9);
    assert!((onsager_energy(CRITICAL_TEMPERATURE) + 2f64.sqrt()).abs() < 1e-6);
    // The energy rises steadily with temperature.
    let mut last = -2.0;
    for k in 1..60 {
        let e = onsager_energy(0.1 * k as f64);
        assert!(e >= last - 1e-12, "energy fell at T = {}", 0.1 * k as f64);
        last = e;
    }
}

#[test]
fn an_ordered_lattice_has_all_its_energy_and_keeps_it_when_cold() {
    let mut lattice = Ising::new(16, 0.0, 0.0, true, 1);
    assert_eq!(lattice.magnetisation(), 1.0);
    assert_eq!(lattice.energy(), -2.0);
    for _ in 0..50 {
        lattice.sweep();
    }
    assert_eq!(
        lattice.magnetisation(),
        1.0,
        "nothing flips at absolute zero"
    );
    // In a field every spin gains −h.
    let fielded = Ising::new(8, 1.0, 0.5, true, 1);
    assert_eq!(fielded.energy(), -2.5);
}

#[test]
fn a_hot_lattice_has_no_magnetisation_and_little_energy() {
    let mut lattice = Ising::new(32, 50.0, 0.0, false, 3);
    let (mut m, mut e) = (0.0, 0.0);
    for _ in 0..300 {
        lattice.sweep();
        m += lattice.magnetisation().abs();
        e += lattice.energy();
    }
    assert!(m / 300.0 < 0.06, "magnetisation {}", m / 300.0);
    // At high temperature the energy per spin is −2 tanh(1/T), nearly.
    assert!((e / 300.0 + 2.0 * (1.0f64 / 50.0).tanh()).abs() < 0.02);
}

#[test]
fn metropolis_reproduces_onsager_away_from_the_critical_point() {
    for temperature in [1.5, 2.0, 3.0, 4.0] {
        let params = Params::default().set("temperature", temperature);
        agree(&run("ising-model", &params, 2200), "ising-model");
    }
}

#[test]
fn a_field_lines_the_spins_up_even_when_hot() {
    let with = run(
        "ising-model",
        &Params::default().set("temperature", 3.5).set("field", 0.8),
        1200,
    );
    let without = run(
        "ising-model",
        &Params::default().set("temperature", 3.5),
        1200,
    );
    let mean = |r: &Run| {
        let soliton_statmech::scene::State::Lattice { lattice, .. } = &r.state else {
            unreachable!()
        };
        lattice.magnetisation()
    };
    assert!(mean(&with) > 0.5 && mean(&without).abs() < 0.2);
}

#[test]
fn cooling_finds_the_critical_temperature() {
    let finished = run("phase-transition", &Params::default(), 20_000);
    assert!(finished.finished());
    agree(&finished, "phase-transition");
    let soliton_statmech::scene::State::Lattice { points, .. } = &finished.state else {
        unreachable!()
    };
    assert_eq!(points.len(), 25);
    // Disordered at the hot end, ordered at the cold end, and each point close to Onsager away from the middle.
    assert!(points[0].magnetisation < 0.2 && points[24].magnetisation > 0.97);
    for point in points
        .iter()
        .filter(|p| (p.temperature - CRITICAL_TEMPERATURE).abs() > 0.5)
    {
        assert!(
            (point.magnetisation - onsager_magnetisation(point.temperature)).abs() < 0.14,
            "{point:?}"
        );
        assert!(
            (point.energy - onsager_energy(point.temperature)).abs() < 0.06,
            "{point:?}"
        );
    }
}

#[test]
fn darts_find_pi_with_the_error_statistics_predicts() {
    let mut darts = Darts::new(100, 7);
    darts.throw(400_000);
    assert_eq!(darts.recent.len(), 100);
    let error = (darts.estimate() - std::f64::consts::PI).abs();
    assert!(error < 3.0 * darts.expected_error(), "off by {error}");
    assert!((darts.expected_error() - 1.642 / (400_000f64).sqrt()).abs() < 1e-5);
    agree(
        &run("monte-carlo-pi", &Params::default(), 2000),
        "monte-carlo-pi",
    );
}

#[test]
fn walkers_spread_as_the_square_root_of_the_steps() {
    let mut walkers = Walkers::new(4000, 5);
    for steps in [100u64, 400, 1600] {
        while walkers.steps < steps {
            walkers.step();
        }
        let expected = (steps as f64).sqrt();
        assert!(
            (walkers.rms_distance() - expected).abs() < 0.05 * expected,
            "after {steps}"
        );
    }
    let finished = run("random-walk", &Params::default(), 5000);
    assert!(finished.finished());
    agree(&finished, "random-walk");
}

#[test]
fn the_same_seed_gives_the_same_run_and_scenes_round_trip() {
    let a = run("ising-model", &Params::default(), 50);
    let b = run("ising-model", &Params::default(), 50);
    assert_eq!(a.history, b.history);
    for scenario in catalogue() {
        let scene = scenario.scene(&Params::default());
        assert_eq!(StatScene::from_json(&scene.to_json()).unwrap(), scene);
    }
    let mut bad = find("ising-model").unwrap().scene(&Params::default());
    bad.playback = 0.0;
    assert!(bad.start().is_err());
}
