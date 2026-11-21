use soliton_matter::fluids::{Capillary, Draining, Falling, Floating, Lift, Venturi};
use soliton_matter::heat::{Conducting, Cooling, Glowing, Heating, Mixing};
use soliton_matter::solids::{Sheared, Squeezed, Wire};
use soliton_matter::{catalogue, find, Lesson, Params};

fn close(found: f64, expected: f64, within: f64) {
    assert!(
        (found - expected).abs() <= within,
        "found {found}, expected {expected} within {within}"
    );
}

fn finish(lesson: &mut dyn Lesson) {
    for _ in 0..40_000 {
        if lesson.finished() {
            return;
        }
        lesson.advance(0.02);
    }
    panic!("it never finished");
}

fn check(lesson: &dyn Lesson) {
    let mut seen = 0;
    for answer in lesson.answers() {
        if let Some(found) = answer.measured {
            close(found, answer.value, answer.within);
            seen += 1;
        }
    }
    assert!(seen > 0, "nothing was measured");
}

#[test]
fn a_wire_obeys_hooke_until_its_elastic_limit() {
    let wire = |load: f64| Wire {
        youngs: 2.0e11,
        length: 2.0,
        diameter: 1e-3,
        load,
        elastic_limit: 2.5e8,
    };
    // 10 kg on a millimetre of steel two metres long stretches it 1.25 mm.
    close(wire(10.0).extension(), 1.248e-3, 2e-6);
    close(wire(20.0).extension(), 2.0 * wire(10.0).extension(), 1e-12);
    assert!(
        wire(10.0).elastic()
            && !wire(25.0).elastic()
            && !wire(25.0).broken()
            && wire(40.0).broken()
    );
    // Past the limit it gives far more for the same extra load.
    assert!(
        wire(25.0).extension() - wire(20.0).extension()
            > 5.0 * (wire(20.0).extension() - wire(15.0).extension())
    );
}

#[test]
fn the_textbooks_sheared_and_squeezed_examples() {
    // A lead block with 90 kN along a 0.05 m² face, half a metre tall: the top moves 0.16 mm.
    let lead = Sheared {
        modulus: 5.6e9,
        force: 9.0e4,
        area: 0.05,
        height: 0.5,
    };
    close(lead.strain() * lead.height, 1.6e-4, 1e-6);
    // Water three kilometres down has lost 1.4 % of its volume.
    close(
        100.0
            * Squeezed {
                modulus: 2.2e9,
                depth: 3000.0,
            }
            .share(),
        1.38,
        0.02,
    );
}

#[test]
fn a_lift_multiplies_force_and_gives_back_the_same_work() {
    let mut lift = Lift::new(100.0, 0.02, 0.2);
    close(lift.gain(), 100.0, 1e-9);
    finish(&mut lift);
    check(&lift);
    close(100.0 * 0.2, 100.0 * lift.gain() * lift.rise(), 1e-9);
}

#[test]
fn a_block_floats_with_the_share_of_it_under_that_archimedes_says() {
    for (block, fluid) in [
        (600.0, 1000.0),
        (917.0, 1030.0),
        (300.0, 1260.0),
        (2700.0, 1000.0),
        (1100.0, 1000.0),
    ] {
        let mut cube = Floating::new(block, fluid);
        finish(&mut cube);
        check(&cube);
        assert_eq!(cube.floats(), block < fluid);
    }
}

#[test]
fn a_speck_is_timed_through_the_throat_at_the_speed_continuity_gives() {
    let mut pipe = Venturi::new(1.5, 3.0, 1000.0);
    for _ in 0..3000 {
        pipe.advance(0.02);
    }
    check(&pipe);
    close(pipe.throat_speed(), 4.5, 1e-12);
    close(pipe.pressure_drop(), 9000.0, 1e-9);
}

#[test]
fn a_tank_empties_in_the_time_torricelli_gives() {
    for (full, hole, tank) in [(1.0, 0.02, 0.4), (2.5, 0.05, 1.0), (0.3, 0.01, 0.2)] {
        let mut draining = Draining::new(full, 1.0, hole, tank);
        finish(&mut draining);
        check(&draining);
    }
    close(Draining::efflux(1.0), 4.427, 1e-3);
    close(Draining::new(1.0, 1.0, 0.02, 0.4).range(1.0), 2.0, 1e-9);
}

#[test]
fn a_ball_reaches_stokes_terminal_speed() {
    let mut ball = Falling::new(2e-3, 7800.0, 1260.0, 0.83);
    close(ball.terminal(), 0.0686, 2e-4);
    finish(&mut ball);
    check(&ball);
    close(ball.speed(), ball.terminal(), 1e-6);
    assert!(ball.reynolds() < 1.0);
}

#[test]
fn water_climbs_a_fine_tube_and_mercury_is_pushed_down() {
    let water = Capillary {
        radius: 0.5e-3,
        tension: 0.073,
        angle: 0.0,
        density: 1000.0,
    };
    close(water.rise(0.5e-3), 0.0298, 1e-4);
    close(water.rise(1.0e-3), 0.5 * water.rise(0.5e-3), 1e-12);
    let mercury = Capillary {
        radius: 0.5e-3,
        tension: 0.465,
        angle: 140.0,
        density: 13_600.0,
    };
    assert!(mercury.rise(0.5e-3) < -0.004);
}

#[test]
fn heat_lost_is_heat_gained() {
    // 200 g of aluminium at 100 °C into 300 g of water at 20 °C.
    let mut mixing = Mixing::new(0.2, 900.0, 100.0, 0.3, 20.0);
    close(mixing.final_temperature(), 30.03, 0.01);
    finish(&mut mixing);
    check(&mixing);
}

#[test]
fn ice_takes_the_latent_heats_to_melt_and_to_boil() {
    let mut heating = Heating::new(0.1, -20.0, 500.0);
    let stages = heating.stages();
    close(stages[0], 4200.0, 1e-9);
    close(stages[1] - stages[0], 33_400.0, 1e-9);
    close(stages[3] - stages[2], 226_000.0, 1e-9);
    assert_eq!(heating.state(stages[0] + 16_700.0), (0.0, [0.5, 0.5, 0.0]));
    finish(&mut heating);
    check(&heating);
    assert_eq!(
        heating
            .answers()
            .iter()
            .filter(|a| a.measured.is_some())
            .count(),
        4
    );
}

#[test]
fn heat_settles_to_a_steady_flow_along_two_metals() {
    for (first, second) in [(385.0, 50.0), (50.0, 385.0), (200.0, 200.0)] {
        let mut rod = Conducting::new(100.0, 0.0, first, second, 1.0, 5e-4);
        finish(&mut rod);
        check(&rod);
    }
    let rod = Conducting::new(100.0, 0.0, 385.0, 50.0, 1.0, 5e-4);
    close(rod.junction(), 88.5, 0.05);
    close(rod.current(), 4.425, 0.005);
}

#[test]
fn cooling_halves_its_excess_in_equal_times() {
    let mut cup = Cooling::new(90.0, 25.0, 0.1);
    finish(&mut cup);
    check(&cup);
    close(cup.temperature(), 25.0 + 65.0 * (-5.0f64).exp(), 1e-6);
}

#[test]
fn plancks_curve_gives_wiens_and_stefans_laws() {
    for temperature in [500.0, 3000.0, 5800.0, 10_000.0] {
        check(&Glowing { temperature });
    }
    close(
        Glowing {
            temperature: 5800.0,
        }
        .brightest()
            * 1e9,
        499.6,
        1.0,
    );
}

#[test]
fn every_scenario_runs_and_its_measurements_agree() {
    for scenario in catalogue() {
        let mut run = scenario.start(&Params::default());
        for _ in 0..6000 {
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
                assert!(
                    (found - answer.value).abs() <= answer.within,
                    "{}: {} found {found}, expected {}",
                    scenario.spec.id,
                    answer.name,
                    answer.value
                );
            }
        }
        assert!(
            run.still() || run.finished() || scenario.spec.id == "bernoulli",
            "{} never finished",
            scenario.spec.id
        );
    }
    assert!(find("buoyancy").is_some() && find("nothing").is_none());
    assert_eq!(catalogue().len(), 16);
}
