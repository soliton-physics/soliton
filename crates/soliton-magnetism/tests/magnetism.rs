use soliton_magnetism::biot_savart::{beside_long_wire, of_turn};
use soliton_magnetism::currents::{
    Conductor, Galvanometer, Loop, Parallel, Solenoid, StraightWire,
};
use soliton_magnetism::induction::{
    Faraday, Frame, Generator, MutualInduction, Rails, SelfInduction, Transformer,
};
use soliton_magnetism::{catalogue, find, Lesson, Params, MU0};
use std::f64::consts::PI;

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

/// Every answer the run has measured agrees with the textbook's.
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
fn biot_savart_summed_along_a_wire_gives_the_textbook_field() {
    // 5 A, 5 cm away: 20 µT.
    let wire = StraightWire::new(5.0, 0.05);
    close(wire.field_at(0.05), 2.0e-5, 1e-12);
    close(wire.summed(), 2.0e-5, 2e-9);
    close(beside_long_wire(0.1), 0.5 * beside_long_wire(0.05), 1e-12);
    check(&wire);
}

#[test]
fn a_coil_is_strongest_at_its_centre_and_a_dipole_far_away() {
    // The textbook's coil: 100 turns of radius 10 cm carrying 1 A has 0.63 mT at its centre.
    let coil = |distance: f64| Loop::new(1.0, 100.0, 0.1, distance);
    close(coil(0.0).on_axis(0.0), 2.0e-4 * PI, 1e-12);
    close(coil(0.0).summed(), 2.0e-4 * PI, 1e-7);
    check(&coil(0.1));
    // Far along the axis it is the field of a dipole of moment NIA: μ₀m/2πx³.
    let far = 5.0;
    close(
        coil(far).summed(),
        MU0 * coil(far).moment() / (2.0 * PI * far.powi(3)),
        1e-3 * coil(far).summed(),
    );
    // Off the axis, in the plane of the coil and outside it, the field points the other way.
    assert!(of_turn(0.1, 0.0, 0.05, 720)[0] > 0.0 && of_turn(0.1, 0.0, 0.2, 720)[0] < 0.0);
}

#[test]
fn a_solenoid_is_even_inside_and_half_as_strong_at_its_end() {
    let coil = |position: f64| Solenoid {
        current: 2.0,
        turns: 500.0,
        length: 0.5,
        radius: 0.02,
        position,
    };
    // 1000 turns a metre at 2 A: 2.51 mT.
    close(coil(0.0).inside(), 2.513e-3, 1e-6);
    close(
        coil(0.0).summed(0.0),
        coil(0.0).inside(),
        0.005 * coil(0.0).inside(),
    );
    close(
        coil(0.0).summed(0.1),
        coil(0.0).summed(0.0),
        0.005 * coil(0.0).inside(),
    );
    close(
        coil(0.0).summed(0.25),
        0.5 * coil(0.0).inside(),
        0.005 * coil(0.0).inside(),
    );
    check(&coil(0.0));
    check(&coil(0.2));
}

#[test]
fn one_ampere_a_metre_apart_is_the_old_definition() {
    let wires = Parallel::new(1.0, 1.0, 1.0);
    close(wires.force(), 2.0e-7, 1e-18);
    assert!(Parallel::new(3.0, -2.0, 0.1).force() < 0.0);
    check(&wires);
    check(&Parallel::new(10.0, -10.0, 0.05));
}

#[test]
fn a_wire_along_the_field_feels_nothing() {
    let wire = |degrees: f64| Conductor {
        current: 5.0,
        length: 0.2,
        field: 0.5,
        angle: degrees.to_radians(),
    };
    close(wire(90.0).force(), 0.5, 1e-12);
    close(wire(30.0).force(), 0.25, 1e-12);
    close(wire(0.0).force(), 0.0, 1e-12);
}

#[test]
fn a_galvanometer_turns_in_proportion_to_the_current() {
    let meter = |current: f64| Galvanometer::new(current, 100.0, 4e-4, 0.2, 8e-7);
    let mut one = meter(50e-6);
    close(one.rest(), 0.5, 1e-12);
    finish(&mut one);
    check(&one);
    let mut two = meter(25e-6);
    finish(&mut two);
    close(two.deflection(), 0.5 * one.deflection(), 1e-4);
}

#[test]
fn the_emf_follows_the_rate_of_change_of_flux() {
    let pass = |speed: f64| {
        let mut run = Faraday::new(1.0, speed, 200.0, 0.03, 10.0);
        finish(&mut run);
        check(&run);
        run.answers()
            .iter()
            .map(|a| a.measured.unwrap())
            .collect::<Vec<_>>()
    };
    let (slow, fast) = (pass(0.5), pass(1.0));
    // Twice as fast: twice the emf, but the same charge, which depends only on the change of flux.
    close(fast[0], 2.0 * slow[0], 2e-3 * fast[0]);
    close(fast[1], slow[1], 2e-3 * slow[1]);
    close(slow[0], 60.0, 0.5);
}

#[test]
fn a_rod_on_rails_reaches_the_speed_where_braking_meets_the_pull() {
    let mut rails = Rails::new(0.5, 0.2, 0.5, 0.05, 0.05);
    close(rails.terminal(), 2.5, 1e-12);
    rails.advance(1.0);
    check(&rails);
    finish(&mut rails);
    check(&rails);
    assert_eq!(
        rails
            .answers()
            .iter()
            .filter(|a| a.measured.is_some())
            .count(),
        2
    );
}

#[test]
fn a_falling_loop_gives_up_as_heat_exactly_what_it_fails_to_gain() {
    let mut braked = Frame::new(0.1, 0.02, 0.02, 0.5);
    finish(&mut braked);
    check(&braked);
    assert!(braked.heat() > 1e-3);
    // With no field it falls five of its own sides freely.
    let mut free = Frame::new(0.1, 0.02, 0.02, 0.0);
    finish(&mut free);
    close(free.heat(), 0.0, 0.0);
    close(free.lost(), 0.0, 1e-9);
    let [first, last] = [&braked.answers()[1], &free.answers()[1]];
    close(last.value, (2.0f64 * 9.8 * 0.5).sqrt(), 1e-9);
    assert!(first.value < last.value);
}

#[test]
fn a_generator_peaks_at_n_b_a_omega() {
    let mut generator = Generator::new(100.0, 0.01, 0.2, 50.0);
    close(generator.peak(), 62.83, 0.01);
    finish(&mut generator);
    check(&generator);
}

#[test]
fn the_work_against_a_back_emf_is_stored_in_the_field() {
    let mut coil = SelfInduction::new(500.0, 0.3, 0.02, 2.0, 0.01);
    close(coil.inductance(), 1.316e-3, 1e-6);
    finish(&mut coil);
    check(&coil);
    assert_eq!(
        coil.answers()
            .iter()
            .filter(|a| a.measured.is_some())
            .count(),
        2
    );

    let mut pair = MutualInduction::new(500.0, 200.0, 0.3, 0.02, 2.0, 50.0);
    close(pair.mutual(), 5.264e-4, 1e-6);
    finish(&mut pair);
    check(&pair);
}

#[test]
fn stepping_the_voltage_up_tenfold_cuts_the_loss_a_hundredfold() {
    let line = Transformer {
        voltage: 4000.0,
        primary: 100.0,
        secondary: 1000.0,
        power: 8.0e5,
        line: 15.0,
    };
    close(line.stepped(), 40_000.0, 1e-9);
    close(line.loss_at(line.voltage), 6.0e5, 1e-6);
    close(line.loss_at(line.stepped()), 6.0e3, 1e-6);
}

#[test]
fn every_scenario_runs_and_agrees_with_its_own_answers() {
    assert_eq!(catalogue().len(), 13);
    for scenario in catalogue() {
        let mut run = scenario.start(&Params::default());
        for _ in 0..40_000 {
            if run.still() || run.finished() {
                break;
            }
            run.advance(0.02);
        }
        assert!(
            run.still() || run.finished(),
            "{} never finished",
            scenario.spec.id
        );
        for reading in run.readings() {
            assert!(
                reading.value.is_finite(),
                "{}: {}",
                scenario.spec.id,
                reading.label
            );
        }
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
        assert!(!run.plots().is_empty(), "{} has no graph", scenario.spec.id);
    }
    assert!(find("motional-emf").is_some() && find("no-such-thing").is_none());
}

#[test]
fn every_scenario_holds_up_at_the_edges_of_what_may_be_typed() {
    for scenario in catalogue() {
        for high in [false, true] {
            let mut params = Params::default();
            for p in scenario.spec.params {
                let (low, top) = soliton_core::typed_limits(p.name, p.unit, p.min, p.max);
                params = params.set(p.name, if high { top } else { low });
            }
            let mut run = scenario.start(&params);
            for _ in 0..2_000 {
                if run.still() || run.finished() {
                    break;
                }
                run.advance(0.02);
            }
            let id = scenario.spec.id;
            assert!(run.still() || run.finished(), "{id} never finished");
            for reading in run.readings() {
                assert!(reading.value.is_finite(), "{id}: {}", reading.label);
            }
            for answer in run.answers() {
                assert!(answer.value.is_finite(), "{id}: {}", answer.name);
                if let Some(found) = answer.measured {
                    assert!(
                        (found - answer.value).abs() <= answer.within,
                        "{id} ({}): {} found {found}, expected {}",
                        if high { "high" } else { "low" },
                        answer.name,
                        answer.value
                    );
                }
            }
            for plot in run.plots() {
                for curve in &plot.curves {
                    assert!(
                        curve.x.iter().chain(&curve.y).all(|v| v.is_finite()),
                        "{id}: {}",
                        plot.title
                    );
                }
            }
            let picture = format!("{:?}", run.picture());
            assert!(
                !picture.contains("NaN") && !picture.contains("inf"),
                "{id}: {picture}"
            );
        }
    }
}
