use num_complex::Complex64;
use soliton_fields::exact::kdv_soliton;
use soliton_fields::{Kdv, PeriodicGrid, Simulate, Simulation};

fn max_error(simulation: &mut Simulation<Kdv>, speed: f64, center: f64) -> f64 {
    let t = simulation.time();
    simulation
        .state()
        .iter()
        .zip(simulation.grid().positions())
        .map(|(u, x)| (u.re - kdv_soliton(x, t, speed, center)).abs())
        .fold(0.0, f64::max)
}

#[test]
fn single_soliton_travels_at_its_speed() {
    let (speed, center) = (4.0, -10.0);
    let mut simulation = Simulation::new(PeriodicGrid::new(512, 60.0), Kdv, 1e-3);
    simulation.set_profile(|x| Complex64::new(kdv_soliton(x, 0.0, speed, center), 0.0));

    simulation.advance(5000);

    assert!(simulation.time() - 5.0 < 1e-9);
    assert!(max_error(&mut simulation, speed, center) < 1e-6);
}

#[test]
fn invariants_are_conserved() {
    let mut simulation = Simulation::new(PeriodicGrid::new(512, 60.0), Kdv, 1e-3);
    simulation.set_profile(|x| {
        Complex64::new(
            kdv_soliton(x, 0.0, 4.0, -15.0) + kdv_soliton(x, 0.0, 1.0, 0.0),
            0.0,
        )
    });
    let before = simulation.invariants();

    simulation.advance(8000);

    for (a, b) in before.iter().zip(simulation.invariants()) {
        let relative = (a.value - b.value).abs() / a.value.abs().max(1.0);
        assert!(relative < 1e-8, "{} drifted by {relative:e}", a.name);
    }
}

#[test]
fn two_solitons_pass_through_each_other() {
    let mut simulation = Simulation::new(PeriodicGrid::new(1024, 80.0), Kdv, 5e-4);
    simulation.set_profile(|x| {
        Complex64::new(
            kdv_soliton(x, 0.0, 6.0, -20.0) + kdv_soliton(x, 0.0, 1.5, 0.0),
            0.0,
        )
    });

    simulation.advance(16000);

    let state = simulation.state();
    let peaks = state.iter().filter(|u| u.re > 0.9 * 0.5 * 1.5).count();
    assert!(
        peaks > 0,
        "slow soliton should still be present after the collision"
    );
    let crest = state.iter().map(|u| u.re).fold(f64::MIN, f64::max);
    assert!(
        (crest - 3.0).abs() < 0.02,
        "fast soliton amplitude {crest} should be restored to 3.0"
    );
}
