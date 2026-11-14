//! The thermal pillar (design 014): what must hold exactly, and each scenario's measured answers against the
//! textbook's, within what a few hundred molecules can be expected to give.

use soliton_thermal::scenario::{catalogue, find, Params};
use soliton_thermal::scene::{maxwell, Experiment, ThermalScene};
use soliton_thermal::{Gas, Molecule, Wall, BOLTZMANN};

fn relative(measured: f64, expected: f64) -> f64 {
    (measured - expected).abs() / expected.abs().max(1e-12)
}

fn start(id: &str, params: &Params) -> Experiment {
    find(id).unwrap().scene(params).start().unwrap()
}

/// Runs a scenario until it finishes, or for `picoseconds` if it never does.
fn run(id: &str, params: &Params, picoseconds: f64) -> Experiment {
    let mut experiment = start(id, params);
    let mut left = picoseconds;
    while left > 0.0 && !experiment.finished() {
        experiment.advance(50.0);
        left -= 50.0;
    }
    experiment
}

/// Asserts that every measured answer is within its own tolerance of the textbook value.
fn agree(experiment: &Experiment, id: &str) {
    let mut checked = 0;
    for (name, value, measured, tolerance) in experiment.answers() {
        let Some(found) = measured else { continue };
        checked += 1;
        // An answer of zero is judged against the scale of the gas's own energy instead.
        let scale = if value == 0.0 { 1000.0 } else { value.abs() };
        assert!(
            (found - value).abs() <= tolerance * scale,
            "{id}, {name}: {found} against {value}"
        );
    }
    assert!(checked > 0, "{id}: nothing was measured");
}

fn boxed(count: usize, seed: u64) -> Gas {
    let mut gas = Gas::new([10.0; 3], seed);
    for _ in 0..count {
        let position = [0; 3].map(|_| 0.5 + 9.0 * gas.rng().uniform());
        let velocity = [0; 3].map(|_| 0.3 * gas.rng().normal());
        gas.molecules.push(Molecule {
            position,
            velocity,
            mass: 40.0,
            radius: 0.15,
            species: 0,
        });
    }
    gas
}

#[test]
fn an_insulated_gas_keeps_its_energy_exactly() {
    let mut gas = boxed(300, 7);
    let before = gas.kinetic_energy();
    for _ in 0..20_000 {
        gas.step(0.05);
    }
    assert!(gas.encounters > 1000, "only {} collisions", gas.encounters);
    assert!(relative(gas.kinetic_energy(), before) < 1e-12);
    assert_eq!(gas.work, 0.0);
    assert_eq!(gas.heat, [0.0; 6]);
    for m in &gas.molecules {
        for axis in 0..3 {
            assert!(
                m.position[axis] > -0.1 && m.position[axis] < 10.1,
                "one got out"
            );
        }
    }
}

#[test]
fn the_same_seed_gives_the_same_gas() {
    let run = |seed| {
        let mut gas = boxed(100, seed);
        gas.walls = [Wall::Bath(300.0); 6];
        for _ in 0..2000 {
            gas.step(0.05);
        }
        gas.molecules
    };
    assert_eq!(run(3), run(3));
    assert_ne!(run(3), run(4));
}

#[test]
fn work_and_heat_account_for_every_change_of_energy() {
    let mut gas = boxed(200, 11);
    gas.walls = [Wall::Bath(450.0); 6];
    let before = gas.kinetic_energy();
    gas.drive_piston(6.0, 0.01);
    for _ in 0..12_000 {
        gas.step(0.05);
    }
    assert_eq!(gas.size[0], 6.0);
    assert!(gas.work > 0.0, "squeezing a gas takes work");
    let heat: f64 = gas.heat.iter().sum();
    // The first law, with nothing left over.
    assert!(relative(gas.kinetic_energy() - before, gas.work + heat) < 1e-9);
}

#[test]
fn a_wall_at_a_temperature_brings_the_gas_to_it() {
    let mut gas = boxed(300, 5);
    gas.walls = [Wall::Bath(500.0); 6];
    let mut sum = 0.0;
    for step in 0..40_000 {
        gas.step(0.05);
        if step >= 20_000 {
            sum += gas.temperature();
        }
    }
    assert!(
        relative(sum / 20_000.0, 500.0) < 0.03,
        "settled at {}",
        sum / 20_000.0
    );
}

#[test]
fn a_partition_keeps_each_gas_to_its_side() {
    let mut experiment = start("mixing", &Params::default());
    experiment.advance(35.0);
    for m in &experiment.gas.molecules {
        assert_eq!(
            m.position[0] < 8.0,
            m.species == 0,
            "a molecule crossed the wall"
        );
    }
}

#[test]
fn maxwells_curve_has_unit_area_and_the_right_mean() {
    let (mass, temperature) = (40.0, 300.0);
    let (mut area, mut mean) = (0.0, 0.0);
    for i in 0..4000 {
        let v = (i as f64 + 0.5) * 0.5;
        area += maxwell(mass, temperature, v) * 0.5;
        mean += v * maxwell(mass, temperature, v) * 0.5;
    }
    assert!(relative(area, 1.0) < 1e-6);
    let expected = (8.0 * BOLTZMANN * temperature / (std::f64::consts::PI * mass)).sqrt() * 1000.0;
    assert!(relative(mean, expected) < 1e-6);
}

#[test]
fn pressure_follows_the_gas_law() {
    for (count, temperature, side) in [
        (300.0, 300.0, 12.0),
        (600.0, 300.0, 12.0),
        (300.0, 600.0, 12.0),
        (300.0, 300.0, 9.0),
    ] {
        let params = Params::default()
            .set("count", count)
            .set("temperature", temperature)
            .set("side", side);
        agree(&run("ideal-gas", &params, 1500.0), "ideal-gas");
    }
}

#[test]
fn equal_speeds_relax_to_maxwells_distribution() {
    let experiment = run("maxwell-distribution", &Params::default(), 1200.0);
    assert_eq!(experiment.stage(), 1);
    agree(&experiment, "maxwell-distribution");
    // At the start every molecule had the root mean square speed, about 433 m/s, and no other.
    let fresh = start("maxwell-distribution", &Params::default());
    let spread = fresh.speed_histogram(Some(0), 40, 1200.0);
    assert_eq!(spread.iter().filter(|d| **d > 0.0).count(), 1);
    let settled = experiment.speed_histogram(Some(0), 12, 1200.0);
    assert!(settled.iter().filter(|d| **d > 0.0).count() >= 8);
}

#[test]
fn light_and_heavy_molecules_come_to_share_one_temperature() {
    let experiment = run("gas-mixture", &Params::default(), 2600.0);
    agree(&experiment, "gas-mixture");
}

#[test]
fn slow_compression_at_a_fixed_temperature_turns_all_the_work_into_heat() {
    let experiment = run("isothermal-process", &Params::default(), 2700.0);
    assert_eq!(experiment.stage(), 2);
    agree(&experiment, "isothermal-process");
}

#[test]
fn slow_compression_with_no_heat_follows_the_adiabat() {
    let experiment = run("adiabatic-process", &Params::default(), 2700.0);
    agree(&experiment, "adiabatic-process");
    // And expanding cools it by the same law.
    let out = Params::default().set("from", 10.0).set("to", 20.0);
    agree(&run("adiabatic-process", &out, 3300.0), "adiabatic-process");
}

#[test]
fn a_gas_expanding_into_nothing_does_no_work_and_stays_as_hot() {
    let experiment = run("free-expansion", &Params::default(), 1500.0);
    agree(&experiment, "free-expansion");
    assert_eq!(experiment.gas.work, 0.0);
    assert_eq!(experiment.gas.size[0], 16.0);
}

#[test]
fn two_gases_mix_evenly_and_stay_mixed() {
    let experiment = run("mixing", &Params::default(), 2400.0);
    agree(&experiment, "mixing");
}

#[test]
fn a_heavy_grain_is_kept_moving_at_the_temperature_of_the_gas() {
    let experiment = run("brownian-motion", &Params::default(), 6000.0);
    agree(&experiment, "brownian-motion");
}

#[test]
fn a_carnot_cycle_is_as_efficient_as_its_two_temperatures_allow() {
    let experiment = run("carnot-cycle", &Params::default(), 20_000.0);
    assert!(
        experiment.finished(),
        "stopped in stage {}",
        experiment.stage()
    );
    agree(&experiment, "carnot-cycle");
    let answers = experiment.answers();
    let efficiency = answers[0].2.unwrap();
    assert!(
        efficiency > 0.28 && efficiency < 0.38,
        "efficiency {efficiency}"
    );
    // The first law over the whole cycle, with the gas's own change of energy counted: nothing is left over.
    let (work, heat): (f64, f64) = experiment
        .records
        .iter()
        .fold((0.0, 0.0), |(w, q), r| (w + r.work, q + r.heat));
    let start = find("carnot-cycle")
        .unwrap()
        .scene(&Params::default())
        .start()
        .unwrap();
    let change = experiment.gas.kinetic_energy() - start.gas.kinetic_energy();
    assert!(
        relative(work + heat, change) < 1e-9,
        "work {work}, heat {heat}, change {change}"
    );
}

#[test]
fn scenes_survive_a_round_trip_through_json_and_bad_ones_are_refused() {
    for scenario in catalogue() {
        let scene = scenario.scene(&Params::default());
        let back = ThermalScene::from_json(&scene.to_json()).unwrap();
        assert_eq!(scene, back, "{}", scenario.spec.id);
    }
    let mut scene = find("ideal-gas").unwrap().scene(&Params::default());
    scene.groups[0].count = 500_000;
    assert!(scene.start().is_err(), "half a million atoms do not fit");
    scene.groups[0].count = 10;
    scene.groups[0].temperature = -1.0;
    assert!(scene.start().is_err());
}
