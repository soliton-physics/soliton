//! Circuits (design 015): Kirchhoff's laws on small circuits whose answers are known, and every scenario's
//! measured answers against the textbook's.

use soliton_circuits::scenario::{catalogue, find, Params};
use soliton_circuits::scene::{Bench, CircuitScene};
use soliton_circuits::{Circuit, Kind};
use std::f64::consts::TAU;

fn relative(measured: f64, expected: f64) -> f64 {
    (measured - expected).abs() / expected.abs().max(1e-12)
}

/// Runs a scenario to the end of whatever time it is followed for.
fn run(id: &str, params: &Params) -> Bench {
    let mut bench = find(id).unwrap().scene(params).start().unwrap();
    for _ in 0..40 {
        if bench.finished() {
            break;
        }
        bench.advance(bench.playback()).unwrap();
    }
    bench
}

/// Asserts that every measured answer agrees with the textbook value.
fn agree(id: &str, params: &Params, tolerance: f64) {
    let bench = run(id, params);
    let mut checked = 0;
    for (name, value, measured) in bench.answers().unwrap() {
        let Some(found) = measured else { continue };
        checked += 1;
        let off = (found - value).abs() / value.abs().max(1e-6);
        assert!(off < tolerance, "{id}, {name}: {found} against {value}");
    }
    assert!(checked > 0, "{id}: nothing was measured");
}

#[test]
fn a_battery_and_a_resistor_obey_ohm() {
    let mut circuit = Circuit::new(2);
    let battery = circuit.add(
        "battery",
        1,
        0,
        Kind::Battery {
            emf: 12.0,
            internal: 1.0,
        },
    );
    let load = circuit.add("load", 1, 0, Kind::Resistor { resistance: 5.0 });
    let state = circuit.steady().unwrap();
    assert!(relative(state.readings[load].current, 2.0) < 1e-12);
    // The terminals are at 12 − 2 × 1 volts, and the cell is supplying: its current runs the other way.
    assert!(relative(state.readings[battery].voltage, 10.0) < 1e-12);
    assert!(relative(state.readings[battery].current, -2.0) < 1e-12);
    let taken: f64 = state.readings.iter().map(|r| r.power()).sum();
    assert!(
        taken.abs() < 1e-12,
        "what the cell supplies, the resistor takes"
    );
}

#[test]
fn currents_into_every_junction_add_to_zero() {
    let bench = run("wheatstone-bridge", &Params::default());
    let mut into = vec![0.0; bench.circuit.nodes];
    for (element, reading) in bench.circuit.elements.iter().zip(&bench.state.readings) {
        into[element.a] -= reading.current;
        into[element.b] += reading.current;
    }
    for (node, sum) in into.iter().enumerate() {
        assert!(sum.abs() < 1e-12, "junction {node} gathers {sum} A");
    }
}

#[test]
fn a_loop_of_ideal_cells_has_no_answer() {
    let mut circuit = Circuit::new(2);
    circuit.add(
        "one",
        1,
        0,
        Kind::Battery {
            emf: 3.0,
            internal: 0.0,
        },
    );
    circuit.add(
        "two",
        1,
        0,
        Kind::Battery {
            emf: 5.0,
            internal: 0.0,
        },
    );
    assert!(circuit.steady().is_none());
}

#[test]
fn a_part_cut_off_by_an_open_switch_carries_nothing() {
    let mut circuit = Circuit::new(3);
    circuit.add(
        "battery",
        1,
        0,
        Kind::Battery {
            emf: 6.0,
            internal: 0.0,
        },
    );
    circuit.add(
        "switch",
        1,
        2,
        Kind::Switch {
            closed: false,
            flips_at: None,
        },
    );
    let lamp = circuit.add("lamp", 2, 0, Kind::Resistor { resistance: 3.0 });
    let off = circuit.steady().unwrap();
    assert_eq!(off.readings[lamp].current, 0.0);
    circuit.elements[1].kind = Kind::Switch {
        closed: true,
        flips_at: None,
    };
    let on = circuit.steady().unwrap();
    assert!(relative(on.readings[lamp].current, 2.0) < 1e-12);
}

#[test]
fn a_capacitor_charges_along_the_exponential() {
    let (emf, r, c) = (9.0, 1e4, 1e-4);
    let mut circuit = Circuit::new(3);
    circuit.add("battery", 1, 0, Kind::Battery { emf, internal: 0.0 });
    circuit.add("R", 1, 2, Kind::Resistor { resistance: r });
    let cap = circuit.add(
        "C",
        2,
        0,
        Kind::Capacitor {
            capacitance: c,
            initial_voltage: 0.0,
        },
    );
    let mut state = circuit.start().unwrap();
    assert!(relative(state.readings[cap].current, emf / r) < 1e-12);
    let dt = 1e-3;
    for _ in 0..3000 {
        state = circuit.step(&state, dt).unwrap();
        let expected = emf * (1.0 - (-state.time / (r * c)).exp());
        assert!((state.readings[cap].voltage - expected).abs() < 1e-6 * emf);
    }
}

#[test]
fn a_switch_thrown_part_way_starts_the_charging_then() {
    let mut circuit = Circuit::new(4);
    circuit.add(
        "battery",
        1,
        0,
        Kind::Battery {
            emf: 5.0,
            internal: 0.0,
        },
    );
    circuit.add(
        "switch",
        1,
        2,
        Kind::Switch {
            closed: false,
            flips_at: Some(0.5),
        },
    );
    circuit.add("R", 2, 3, Kind::Resistor { resistance: 1000.0 });
    let cap = circuit.add(
        "C",
        3,
        0,
        Kind::Capacitor {
            capacitance: 1e-3,
            initial_voltage: 0.0,
        },
    );
    let mut state = circuit.start().unwrap();
    for _ in 0..1500 {
        state = circuit.step(&state, 1e-3).unwrap();
        let since = state.time - 0.5;
        let expected = if since < 0.0 {
            0.0
        } else {
            5.0 * (1.0 - (-since).exp())
        };
        assert!(
            (state.readings[cap].voltage - expected).abs() < 5e-3,
            "at {}",
            state.time
        );
    }
}

#[test]
fn an_lc_circuit_keeps_its_energy_and_its_period() {
    let (l, c, v0) = (0.1, 1e-4, 10.0);
    let mut circuit = Circuit::new(2);
    let cap = circuit.add(
        "C",
        1,
        0,
        Kind::Capacitor {
            capacitance: c,
            initial_voltage: v0,
        },
    );
    let coil = circuit.add(
        "L",
        1,
        0,
        Kind::Inductor {
            inductance: l,
            initial_current: 0.0,
        },
    );
    let period = TAU * (l * c).sqrt();
    let dt = period / 2000.0;
    let mut state = circuit.start().unwrap();
    let energy = |s: &soliton_circuits::State| {
        0.5 * c * s.readings[cap].voltage.powi(2) + 0.5 * l * s.readings[coil].current.powi(2)
    };
    let start = energy(&state);
    for _ in 0..10_000 {
        state = circuit.step(&state, dt).unwrap();
    }
    // Five whole periods on, it is where it began.
    assert!(relative(energy(&state), start) < 1e-9);
    assert!(relative(state.readings[cap].voltage, v0) < 1e-4);
}

#[test]
fn alternating_current_is_the_same_by_complex_numbers_and_by_stepping() {
    let (r, l, c, f) = (20.0, 0.2, 1e-4, 50.0);
    let mut circuit = Circuit::new(4);
    circuit.add(
        "source",
        1,
        0,
        Kind::Source {
            amplitude: 100.0,
            frequency: f,
            phase: 0.0,
        },
    );
    let res = circuit.add("R", 1, 2, Kind::Resistor { resistance: r });
    circuit.add(
        "L",
        2,
        3,
        Kind::Inductor {
            inductance: l,
            initial_current: 0.0,
        },
    );
    circuit.add(
        "C",
        3,
        0,
        Kind::Capacitor {
            capacitance: c,
            initial_voltage: 0.0,
        },
    );
    let phasors = circuit.phasors().unwrap();
    let (xl, xc) = (TAU * f * l, 1.0 / (TAU * f * c));
    assert!(relative(phasors[res].current.norm(), 100.0 / r.hypot(xl - xc)) < 1e-12);
    // Switched on and followed until the start has died away, it settles to the same amplitude.
    let dt = 1.0 / (f * 2000.0);
    let mut state = circuit.start().unwrap();
    let mut peak = 0.0f64;
    for step in 0..60_000 {
        state = circuit.step(&state, dt).unwrap();
        if step >= 56_000 {
            peak = peak.max(state.readings[res].current.abs());
        }
    }
    assert!(relative(peak, phasors[res].current.norm()) < 1e-4, "{peak}");
}

#[test]
fn every_scenario_measures_what_the_textbook_predicts() {
    for scenario in catalogue() {
        let tolerance = match scenario.spec.id {
            // At the peak the diode is charging the capacitor as well as feeding the load, and drops more.
            "rectifier-filter" => 2e-2,
            _ => 1e-3,
        };
        agree(scenario.spec.id, &Params::default(), tolerance);
    }
}

#[test]
fn the_bridge_balances_where_the_ratio_says() {
    let balanced = Params::default().set("s", 30.0);
    let bench = run("wheatstone-bridge", &balanced);
    let answers = bench.answers().unwrap();
    assert!(answers[0].2.unwrap().abs() < 1e-9, "{:?}", answers[0]);
    assert_eq!(answers[2].1, 1.0);
    for s in [5.0, 30.0, 80.0] {
        agree("wheatstone-bridge", &Params::default().set("s", s), 1e-9);
    }
    // The metre bridge balances at 100R/(R + S) = 40 cm, and the reading changes sign across it.
    let at = |jockey: f64| {
        run("meter-bridge", &Params::default().set("jockey", jockey))
            .answers()
            .unwrap()[0]
            .2
            .unwrap()
    };
    assert!(at(40.0).abs() < 1e-9);
    assert!(at(30.0) * at(50.0) < 0.0);
    // The potentiometer at 100ε/V = 37.5 cm.
    let pot = |jockey: f64| {
        run("potentiometer", &Params::default().set("jockey", jockey))
            .answers()
            .unwrap()[0]
            .2
            .unwrap()
    };
    assert!(pot(37.5).abs() < 1e-9);
    assert!(pot(30.0) * pot(45.0) < 0.0);
    agree(
        "potentiometer",
        &Params::default().set("jockey", 80.0),
        1e-9,
    );
}

#[test]
fn steady_scenarios_hold_across_their_sliders() {
    agree(
        "ohms-law",
        &Params::default().set("emf", 24.0).set("resistance", 3.0),
        1e-12,
    );
    agree(
        "resistors-in-series",
        &Params::default().set("third", 50.0),
        1e-12,
    );
    agree(
        "resistors-in-parallel",
        &Params::default().set("first", 50.0),
        1e-12,
    );
    agree(
        "internal-resistance",
        &Params::default().set("load", 0.5),
        1e-12,
    );
    agree(
        "kirchhoff-rules",
        &Params::default().set("right", 20.0).set("third", 1.0),
        1e-12,
    );
    // A weak cell beside a strong one is driven backwards.
    let lopsided = Params::default().set("first", 0.5).set("second", 6.0);
    agree("cells-in-parallel", &lopsided, 1e-12);
    let answers = run("cells-in-parallel", &lopsided).answers().unwrap();
    assert!(answers[3].2.unwrap() < 0.0, "{:?}", answers[3]);
}

#[test]
fn an_lcr_circuit_is_a_plain_resistance_at_resonance() {
    let scene = find("series-lcr").unwrap().scene(&Params::default());
    let bench = scene.start().unwrap();
    let resonant = bench
        .answers()
        .unwrap()
        .iter()
        .find(|a| a.0.starts_with("Resonant"))
        .unwrap()
        .1;
    let tuned = run("series-lcr", &Params::default().set("frequency", resonant));
    let answers = tuned.answers().unwrap();
    let get = |start: &str| {
        answers
            .iter()
            .find(|a| a.0.starts_with(start))
            .unwrap()
            .2
            .unwrap()
    };
    assert!(relative(get("Impedance"), 20.0) < 1e-3);
    assert!(get("Current behind").abs() < 1.0);
    // The voltages across L and C are each more than twice the source's, and equal.
    assert!(get("Peak voltage across the inductor") > 200.0);
    assert!(
        relative(
            get("Peak voltage across the inductor"),
            get("Peak voltage across the capacitor")
        ) < 1e-2
    );
    for frequency in [10.0, 50.0, 200.0] {
        agree(
            "series-lcr",
            &Params::default().set("frequency", frequency),
            1e-9,
        );
    }
}

#[test]
fn scenes_survive_a_round_trip_through_json_and_bad_ones_are_refused() {
    for scenario in catalogue() {
        let scene = scenario.scene(&Params::default());
        let back = CircuitScene::from_json(&scene.to_json()).unwrap();
        assert_eq!(scene, back, "{}", scenario.spec.id);
    }
    let mut scene = find("ohms-law").unwrap().scene(&Params::default());
    scene.elements[3].a = 99;
    assert!(scene.start().is_err());
    let mut shorted = find("ohms-law").unwrap().scene(&Params::default());
    shorted.elements.retain(|e| e.name != "R");
    shorted.elements.push(soliton_circuits::scene::ElementSpec {
        name: "short".to_string(),
        a: 0,
        b: 1,
        kind: soliton_circuits::scene::KindSpec::Wire,
    });
    assert!(
        shorted.start().is_err(),
        "an ideal battery across a wire has no answer"
    );
}

#[test]
fn a_diode_follows_shockleys_equation() {
    use soliton_circuits::{junction, SILICON, THERMAL_VOLTAGE};
    // Sixty millivolts more across it for every tenfold in the current.
    let at = |v: f64| junction(SILICON, None, v).0;
    assert!((at(0.66) / at(0.60) - (0.06 / THERMAL_VOLTAGE).exp()).abs() < 1e-3);
    assert!((at(-5.0) + SILICON).abs() < 1e-10);
    // A Zener takes its test current backwards at its rated voltage.
    assert!((junction(SILICON, Some(5.1), -5.1).0 + 5e-3).abs() < 1e-5);
    for volts in [-8.0, -0.5, 0.3, 0.75, 1.5, 5.0] {
        agree(
            "pn-junction",
            &Params::default().set("voltage", volts),
            1e-6,
        );
        let germanium = Params::default()
            .set("voltage", volts)
            .set("germanium", 1.0);
        agree("pn-junction", &germanium, 1e-6);
    }
    // Silicon needs about 0.7 V to pass a few milliamperes, and germanium about 0.2 V.
    let drop = |germanium: f64| {
        let params = Params::default()
            .set("voltage", 1.5)
            .set("germanium", germanium);
        run("pn-junction", &params).answers().unwrap()[0].2.unwrap()
    };
    assert!((drop(0.0) - 0.62).abs() < 0.06, "{}", drop(0.0));
    assert!((drop(1.0) - 0.22).abs() < 0.06, "{}", drop(1.0));
}

fn load_voltages(id: &str, params: &Params) -> Vec<f64> {
    let mut bench = run(id, params);
    while !bench.finished() {
        bench.advance(1e-3).unwrap();
    }
    let load = bench.circuit.find("load").unwrap();
    bench
        .history
        .iter()
        .map(|s| s.readings[load].voltage)
        .collect()
}

#[test]
fn rectifiers_pass_current_one_way() {
    let half = load_voltages("half-wave-rectifier", &Params::default());
    let full = load_voltages("full-wave-rectifier", &Params::default());
    let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
    // Never backwards, and the full-wave rectifier delivers twice the average of the half-wave.
    assert!(half.iter().chain(&full).all(|v| *v > -1e-6));
    assert!(
        (mean(&full) / mean(&half) - 2.0).abs() < 0.02,
        "{} {}",
        mean(&full),
        mean(&half)
    );
    assert!(
        (mean(&half) - 9.3 / std::f64::consts::PI).abs() < 0.2,
        "{}",
        mean(&half)
    );
    // Half the time the half-wave rectifier gives nothing at all.
    let idle = half.iter().filter(|v| **v < 1e-3).count() as f64 / half.len() as f64;
    assert!((idle - 0.5).abs() < 0.06, "{idle}");
}

#[test]
fn a_capacitor_smooths_the_output_and_a_bigger_one_smooths_it_more() {
    let ripple = |microfarads: f64| {
        let v = load_voltages(
            "rectifier-filter",
            &Params::default().set("capacitance", microfarads),
        );
        // After the first two cycles, when it has charged.
        let settled = &v[v.len() * 2 / 5..];
        settled.iter().cloned().fold(f64::MIN, f64::max)
            - settled.iter().cloned().fold(f64::MAX, f64::min)
    };
    let (small, large) = (ripple(100.0), ripple(400.0));
    // I/(2fC) with 9.3 V on 1 kΩ: 0.93 V for 100 µF. The estimate is on the high side, as it always is.
    assert!(small > 0.6 && small < 0.95, "{small}");
    assert!((small / large - 4.0).abs() < 0.6, "{small} {large}");
}

#[test]
fn a_zener_holds_its_voltage_while_the_input_changes() {
    let output = |input: f64, load: f64| {
        let params = Params::default().set("input", input).set("load", load);
        run("zener-regulator", &params).answers().unwrap()[0]
            .2
            .unwrap()
    };
    // From 9 V to 20 V in, the output moves by under a tenth of a volt.
    assert!(
        (output(9.0, 1.0) - 5.1).abs() < 0.08,
        "{}",
        output(9.0, 1.0)
    );
    assert!((output(20.0, 1.0) - output(9.0, 1.0)).abs() < 0.1);
    assert!((output(12.0, 5.0) - output(12.0, 0.5)).abs() < 0.1);
    // With too little input it is only a potential divider.
    assert!((output(6.0, 0.2) - 6.0 * 200.0 / 530.0).abs() < 1e-3);
    for input in [6.0, 9.0, 12.0, 20.0] {
        agree(
            "zener-regulator",
            &Params::default().set("input", input),
            1e-6,
        );
    }
}

#[test]
fn diode_gates_have_the_right_truth_tables() {
    for (a, b) in [(0.0, 0.0), (0.0, 1.0), (1.0, 0.0), (1.0, 1.0)] {
        let params = Params::default().set("a", a).set("b", b);
        for (id, expected) in [("or-gate", a + b > 0.0), ("and-gate", a * b > 0.0)] {
            let answers = run(id, &params).answers().unwrap();
            assert_eq!(answers[0].1 > 0.5, expected, "{id} {a} {b}");
            // High is within a diode's drop of 5 V and low within one of 0 V.
            let volts = answers[1].2.unwrap();
            assert_eq!(volts > 2.5, expected, "{id} {a} {b}: {volts}");
            assert!(volts > 4.2 || volts < 0.8, "{id} {a} {b}: {volts}");
            agree(id, &params, 1e-6);
        }
    }
}

#[test]
fn a_capacitor_straight_across_a_cell_is_charged_at_once_and_then_carries_nothing() {
    let mut circuit = Circuit::new(2);
    circuit.add(
        "cell",
        1,
        0,
        Kind::Battery {
            emf: 6.0,
            internal: 0.0,
        },
    );
    circuit.add("R", 1, 0, Kind::Resistor { resistance: 100.0 });
    let c = circuit.add(
        "C",
        1,
        0,
        Kind::Capacitor {
            capacitance: 1e-4,
            initial_voltage: 0.0,
        },
    );
    let mut state = circuit.start().unwrap();
    for _ in 0..50 {
        state = circuit.step(&state, 1e-5).unwrap();
        assert!((state.readings[c].voltage - 6.0).abs() < 1e-6);
        assert!(
            state.readings[c].current.abs() < 1e-6,
            "{}",
            state.readings[c].current
        );
    }
}
