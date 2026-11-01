use soliton_core::exact::nls_bright_soliton;
use soliton_core::{Nls, PeriodicGrid, Simulation};

#[test]
fn moving_bright_soliton_matches_exact_solution() {
    let (amplitude, velocity, center) = (1.0, 1.0, -8.0);
    let mut simulation = Simulation::new(
        PeriodicGrid::new(512, 40.0 * std::f64::consts::PI),
        Nls,
        1e-3,
    );
    simulation.set_profile(|x| nls_bright_soliton(x, 0.0, amplitude, velocity, center));

    simulation.advance(10000);

    let t = simulation.time();
    let error = simulation
        .state()
        .iter()
        .zip(simulation.grid().positions())
        .map(|(psi, x)| (psi - nls_bright_soliton(x, t, amplitude, velocity, center)).norm())
        .fold(0.0, f64::max);
    assert!(error < 1e-6, "max error {error:e}");
}

#[test]
fn invariants_are_conserved() {
    let mut simulation = Simulation::new(
        PeriodicGrid::new(512, 40.0 * std::f64::consts::PI),
        Nls,
        1e-3,
    );
    simulation.set_profile(|x| {
        nls_bright_soliton(x, 0.0, 1.0, 1.0, -15.0) + nls_bright_soliton(x, 0.0, 0.7, -0.5, 10.0)
    });
    let before = simulation.invariants();

    simulation.advance(10000);

    for (a, b) in before.iter().zip(simulation.invariants()) {
        let relative = (a.value - b.value).abs() / a.value.abs().max(1.0);
        assert!(relative < 1e-8, "{} drifted by {relative:e}", a.name);
    }
}
