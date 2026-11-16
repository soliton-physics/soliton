use soliton_modern::atom::{energy, radius, wavelength};
use soliton_modern::matter::{electron_wavelength, Diffraction};
use soliton_modern::nucleus::{fusion_energy, liquid_drop, most_stable, Nuclide, Sample, MEASURED};
use soliton_modern::photoelectric::{maximum_energy, Tube};
use soliton_modern::rutherford::{closest_approach, fly, scattering_angle};
use soliton_modern::{catalogue, find, Lesson, Params};

fn close(found: f64, expected: f64, within: f64) {
    assert!(
        (found - expected).abs() <= within,
        "found {found}, expected {expected} within {within}"
    );
}

#[test]
fn einsteins_equation() {
    // Caesium, work function 2.14 eV: threshold 579 nm, and 400 nm light frees electrons of up to 0.96 eV.
    close(maximum_energy(400.0, 2.14), 0.9596, 1e-3);
    close(maximum_energy(579.37, 2.14), 0.0, 1e-3);
    assert!(maximum_energy(650.0, 2.14) < 0.0);
}

#[test]
fn the_collector_counts_what_einstein_predicts() {
    for (voltage, share) in [(1.0, 1.0), (0.0, 1.0), (-0.48, 0.5), (-1.2, 0.0)] {
        let mut tube = Tube::new(400.0, 100.0, voltage, 2.14, 7);
        for _ in 0..6000 {
            tube.advance(0.02);
        }
        let found = tube.measured_current().expect("it has run long enough");
        close(found, 40.0 * share, 2.5);
        close(tube.expected_current(), 40.0 * share, 0.2);
    }
}

#[test]
fn no_electrons_below_the_threshold_however_bright() {
    let mut tube = Tube::new(650.0, 100.0, 4.0, 2.14, 7);
    for _ in 0..3000 {
        tube.advance(0.02);
    }
    assert_eq!(tube.measured_current(), Some(0.0));
}

#[test]
fn de_broglie_and_davisson_germer() {
    close(electron_wavelength(100.0), 0.12264, 1e-4);
    close(electron_wavelength(54.0), 0.1669, 1e-4);
    let nickel = Diffraction {
        voltage: 54.0,
        spacing: 0.215,
    };
    close(nickel.peak().unwrap(), 50.9, 0.2);
    close(nickel.found_peak().unwrap(), nickel.peak().unwrap(), 1.5);
    assert!(Diffraction {
        voltage: 20.0,
        spacing: 0.215
    }
    .peak()
    .is_none());
}

#[test]
fn rutherfords_formula_from_newtons_law() {
    // The textbook's example: a 7.7 MeV alpha comes within 30 fm of a gold nucleus.
    close(closest_approach(7.7, 79.0), 29.55, 0.05);
    let head_on = fly(7.7, 79.0, 0.0);
    close(head_on.nearest, 29.55, 0.1);
    close(head_on.turned, 180.0, 1e-6);
    for impact in [10.0, 30.0, 80.0, 200.0] {
        let flight = fly(5.5, 79.0, impact);
        close(flight.turned, scattering_angle(5.5, 79.0, impact), 0.15);
    }
    // Turned through a right angle when the impact parameter is half the closest approach.
    close(
        scattering_angle(5.5, 79.0, closest_approach(5.5, 79.0) / 2.0),
        90.0,
        1e-9,
    );
}

#[test]
fn bohrs_atom() {
    close(energy(1.0, 1), -13.606, 1e-3);
    close(energy(1.0, 2), -3.401, 1e-3);
    close(radius(1.0, 1), 0.0529, 1e-4);
    close(wavelength(1.0, 3, 2), 656.3, 0.5);
    close(wavelength(1.0, 2, 1), 121.5, 0.2);
    // The Balmer limit, and helium's ion with four times the energies.
    close(wavelength(1.0, 100_000, 2), 364.5, 0.2);
    close(energy(2.0, 1), -54.42, 0.01);
}

#[test]
fn decay_halves_in_each_half_life() {
    let mut sample = Sample::new(2500, 4.0, 11);
    while !sample.finished() {
        sample.advance(0.05);
    }
    close(sample.left_at(4.0).unwrap(), 1250.0, 75.0);
    close(sample.left_at(8.0).unwrap(), 625.0, 65.0);
    close(
        sample.mean_life().unwrap(),
        4.0 / std::f64::consts::LN_2,
        0.4,
    );
    for answer in sample.answers() {
        let found = answer.measured.expect("all measured by the end");
        close(found, answer.value, answer.within);
    }
}

#[test]
fn the_binding_energy_curve() {
    assert_eq!(most_stable(56), 26);
    close(liquid_drop(56, 26) / 56.0, 8.79, 0.1);
    close(liquid_drop(238, 92) / 238.0, 7.57, 0.1);
    // The formula follows the measured values for all but the lightest nuclei.
    for (a, z, measured) in MEASURED.iter().filter(|m| m.0 >= 40) {
        close(liquid_drop(*a, *z) / *a as f64, *measured, 0.12);
    }
    close(fusion_energy(), 17.59, 0.02);
    // Heavy nuclei release energy by splitting and light ones by fusing; near iron neither pays.
    assert!((150.0..250.0).contains(&Nuclide { a: 236 }.fission()));
    assert!(Nuclide { a: 20 }.fusion() > 0.0 && Nuclide { a: 20 }.fission() < 0.0);
    assert!(Nuclide { a: 200 }.fusion() < 0.0);
}

#[test]
fn every_scenario_runs_and_its_measurements_agree() {
    for scenario in catalogue() {
        let mut run = scenario.start(&Params::default());
        for _ in 0..4000 {
            run.advance(0.02);
        }
        assert!(!run.readings().is_empty(), "{}", scenario.spec.id);
        let _ = (run.picture(), run.plots(), run.clock());
        for answer in run.answers() {
            assert!(
                answer.value.is_finite(),
                "{}: {}",
                scenario.spec.id,
                answer.name
            );
            if let Some(found) = answer.measured {
                close(found, answer.value, answer.within);
            }
        }
    }
    assert!(find("bohr-model").is_some() && find("nothing").is_none());
    assert_eq!(catalogue().len(), 8);
}
