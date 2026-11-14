//! The catalogue: NCERT class 12, current electricity, capacitors, and alternating current (design 015). Each
//! scenario is a function from a few parameters to a circuit laid out on the page, with the textbook's answer
//! beside what Kirchhoff's laws give.

use crate::scene::{
    Analysis, AnswerSpec, CircuitScene, ElementSpec, KindSpec, MeasureSpec, Quantity, SyllabusSpec,
    SCHEMA,
};
use serde::Serialize;
use std::f64::consts::TAU;

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Param {
    pub name: &'static str,
    pub label: &'static str,
    pub unit: &'static str,
    pub min: f64,
    pub max: f64,
    pub step: f64,
    pub default: f64,
}

const fn param(
    name: &'static str,
    label: &'static str,
    unit: &'static str,
    range: (f64, f64),
    step: f64,
    default: f64,
) -> Param {
    Param {
        name,
        label,
        unit,
        min: range.0,
        max: range.1,
        step,
        default,
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Syllabus {
    pub class: u8,
    pub chapter: &'static str,
    pub topic: &'static str,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct ScenarioSpec {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub syllabus: Syllabus,
    pub params: &'static [Param],
}

/// Chosen parameter values; anything not set takes the spec's default.
#[derive(Clone, Debug, Default)]
pub struct Params {
    values: Vec<(String, f64)>,
}

impl Params {
    pub fn set(mut self, name: &str, value: f64) -> Self {
        self.values.retain(|(n, _)| n != name);
        self.values.push((name.to_string(), value));
        self
    }

    fn get(&self, spec: &ScenarioSpec, name: &str) -> f64 {
        let param = spec
            .params
            .iter()
            .find(|p| p.name == name)
            .unwrap_or_else(|| panic!("scenario `{}` has no parameter `{name}`", spec.id));
        self.values
            .iter()
            .find(|(n, _)| n == name)
            .map_or(param.default, |(_, v)| v.clamp(param.min, param.max))
    }
}

pub struct Scenario {
    pub spec: ScenarioSpec,
    describe: fn(&ScenarioSpec, &Params) -> CircuitScene,
}

impl Scenario {
    pub fn scene(&self, params: &Params) -> CircuitScene {
        let spec = &self.spec;
        let mut scene = (self.describe)(spec, params);
        scene.description = Some(spec.description.to_string());
        scene.syllabus = Some(SyllabusSpec {
            class: spec.syllabus.class,
            chapter: spec.syllabus.chapter.to_string(),
            topic: spec.syllabus.topic.to_string(),
        });
        scene
    }
}

pub fn catalogue() -> &'static [Scenario] {
    CATALOGUE
}

pub fn find(id: &str) -> Option<&'static Scenario> {
    CATALOGUE.iter().find(|s| s.spec.id == id)
}

type Point = (f64, f64);

/// A circuit being drawn: components are put between points on the page, and points at the same place are the
/// same junction.
struct Board {
    scene: CircuitScene,
}

impl Board {
    fn new(spec: &ScenarioSpec, analysis: Analysis) -> Self {
        Self {
            scene: CircuitScene {
                schema: SCHEMA,
                title: spec.title.to_string(),
                description: None,
                syllabus: None,
                nodes: Vec::new(),
                elements: Vec::new(),
                analysis,
                watch: Vec::new(),
                answers: Vec::new(),
            },
        }
    }

    fn node(&mut self, at: Point) -> usize {
        let found = self
            .scene
            .nodes
            .iter()
            .position(|n| (n[0] - at.0).abs() < 1e-9 && (n[1] - at.1).abs() < 1e-9);
        found.unwrap_or_else(|| {
            self.scene.nodes.push([at.0, at.1]);
            self.scene.nodes.len() - 1
        })
    }

    /// A component from `a` to `b`: its voltage is that of `a` above `b`, its current runs from `a` to `b`.
    fn part(mut self, name: &str, a: Point, b: Point, kind: KindSpec) -> Self {
        let (a, b) = (self.node(a), self.node(b));
        self.scene.elements.push(ElementSpec {
            name: name.to_string(),
            a,
            b,
            kind,
        });
        self
    }

    /// Wire through the points in turn.
    fn wire(mut self, through: &[Point]) -> Self {
        for pair in through.windows(2) {
            let name = format!("wire {}", self.scene.elements.len());
            self = self.part(&name, pair[0], pair[1], KindSpec::Wire);
        }
        self
    }

    fn watch(mut self, names: &[&str]) -> Self {
        self.scene.watch = names.iter().map(|n| n.to_string()).collect();
        self
    }

    fn answer(mut self, name: &str, value: f64) -> Self {
        self.scene.answers.push(AnswerSpec {
            name: name.to_string(),
            value,
            measure: None,
            scale: 1.0,
        });
        self
    }

    fn measured(mut self, name: &str, value: f64, measure: MeasureSpec, scale: f64) -> Self {
        self.scene.answers.push(AnswerSpec {
            name: name.to_string(),
            value,
            measure: Some(measure),
            scale,
        });
        self
    }

    fn done(self) -> CircuitScene {
        self.scene
    }
}

fn resistor(resistance: f64) -> KindSpec {
    KindSpec::Resistor {
        resistance,
        length_of_wire: false,
    }
}

/// Part of a bridge or potentiometer wire.
fn wire_of(resistance: f64) -> KindSpec {
    KindSpec::Resistor {
        resistance,
        length_of_wire: true,
    }
}

fn cell(emf: f64, internal: f64) -> KindSpec {
    KindSpec::Battery { emf, internal }
}

fn current(element: &str) -> MeasureSpec {
    MeasureSpec::Current {
        element: element.to_string(),
    }
}

fn voltage(element: &str) -> MeasureSpec {
    MeasureSpec::Voltage {
        element: element.to_string(),
    }
}

fn power(element: &str) -> MeasureSpec {
    MeasureSpec::Power {
        element: element.to_string(),
    }
}

const MILLI: f64 = 1e3;
const MICRO: f64 = 1e6;

// ---------------------------------------------------------------- current electricity

static OHM: [Param; 2] = [
    param("emf", "Battery", "V", (1.0, 24.0), 0.5, 6.0),
    param("resistance", "Resistance", "Ω", (1.0, 100.0), 1.0, 12.0),
];

fn ohms_law(spec: &ScenarioSpec, p: &Params) -> CircuitScene {
    let (emf, r) = (p.get(spec, "emf"), p.get(spec, "resistance"));
    Board::new(spec, Analysis::Steady)
        .part("battery", (0.0, 2.0), (0.0, 0.0), cell(emf, 0.0))
        .part("A", (0.0, 2.0), (2.0, 2.0), KindSpec::Ammeter)
        .wire(&[(2.0, 2.0), (4.0, 2.0)])
        .part("R", (4.0, 2.0), (4.0, 0.0), resistor(r))
        .wire(&[(4.0, 0.0), (0.0, 0.0)])
        .wire(&[(4.0, 2.0), (5.5, 2.0)])
        .part("V", (5.5, 2.0), (5.5, 0.0), KindSpec::Voltmeter)
        .wire(&[(5.5, 0.0), (4.0, 0.0)])
        .measured("Current, V/R (A)", emf / r, current("A"), 1.0)
        .measured("Voltage across the resistor (V)", emf, voltage("V"), 1.0)
        .measured(
            "Power turned to heat, V²/R (W)",
            emf * emf / r,
            power("R"),
            1.0,
        )
        .done()
}

static THREE: [Param; 4] = [
    param("emf", "Battery", "V", (1.0, 24.0), 0.5, 12.0),
    param("first", "R₁", "Ω", (1.0, 50.0), 1.0, 2.0),
    param("second", "R₂", "Ω", (1.0, 50.0), 1.0, 4.0),
    param("third", "R₃", "Ω", (1.0, 50.0), 1.0, 6.0),
];

fn series(spec: &ScenarioSpec, p: &Params) -> CircuitScene {
    let emf = p.get(spec, "emf");
    let r = [
        p.get(spec, "first"),
        p.get(spec, "second"),
        p.get(spec, "third"),
    ];
    let total: f64 = r.iter().sum();
    let i = emf / total;
    Board::new(spec, Analysis::Steady)
        .part("battery", (0.0, 2.0), (0.0, 0.0), cell(emf, 0.0))
        .part("A", (0.0, 2.0), (1.5, 2.0), KindSpec::Ammeter)
        .part("R₁", (1.5, 2.0), (3.5, 2.0), resistor(r[0]))
        .part("R₂", (3.5, 2.0), (5.5, 2.0), resistor(r[1]))
        .wire(&[(5.5, 2.0), (6.5, 2.0)])
        .part("R₃", (6.5, 2.0), (6.5, 0.0), resistor(r[2]))
        .wire(&[(6.5, 0.0), (0.0, 0.0)])
        .answer("Equivalent resistance, R₁ + R₂ + R₃ (Ω)", total)
        .measured("Current, the same in all three (A)", i, current("A"), 1.0)
        .measured("Voltage across R₁ (V)", i * r[0], voltage("R₁"), 1.0)
        .measured("Voltage across R₂ (V)", i * r[1], voltage("R₂"), 1.0)
        .measured("Voltage across R₃ (V)", i * r[2], voltage("R₃"), 1.0)
        .done()
}

fn parallel(spec: &ScenarioSpec, p: &Params) -> CircuitScene {
    let emf = p.get(spec, "emf");
    let r = [
        p.get(spec, "first"),
        p.get(spec, "second"),
        p.get(spec, "third"),
    ];
    let together = 1.0 / r.iter().map(|x| 1.0 / x).sum::<f64>();
    Board::new(spec, Analysis::Steady)
        .part("battery", (0.0, 2.0), (0.0, 0.0), cell(emf, 0.0))
        .part("A", (0.0, 2.0), (2.0, 2.0), KindSpec::Ammeter)
        .wire(&[(2.0, 2.0), (3.5, 2.0), (5.0, 2.0)])
        .part("R₁", (2.0, 2.0), (2.0, 0.0), resistor(r[0]))
        .part("R₂", (3.5, 2.0), (3.5, 0.0), resistor(r[1]))
        .part("R₃", (5.0, 2.0), (5.0, 0.0), resistor(r[2]))
        .wire(&[(5.0, 0.0), (3.5, 0.0), (2.0, 0.0), (0.0, 0.0)])
        .answer(
            "Equivalent resistance, 1/(1/R₁ + 1/R₂ + 1/R₃) (Ω)",
            together,
        )
        .measured(
            "Current from the battery (A)",
            emf / together,
            current("A"),
            1.0,
        )
        .measured("Current in R₁ (A)", emf / r[0], current("R₁"), 1.0)
        .measured("Current in R₂ (A)", emf / r[1], current("R₂"), 1.0)
        .measured("Current in R₃ (A)", emf / r[2], current("R₃"), 1.0)
        .done()
}

static INTERNAL: [Param; 3] = [
    param("emf", "EMF of the cell", "V", (1.0, 12.0), 0.1, 1.5),
    param(
        "internal",
        "Its internal resistance",
        "Ω",
        (0.1, 5.0),
        0.1,
        0.5,
    ),
    param(
        "load",
        "Resistance connected to it",
        "Ω",
        (0.1, 20.0),
        0.1,
        2.5,
    ),
];

fn internal_resistance(spec: &ScenarioSpec, p: &Params) -> CircuitScene {
    let (emf, r, load) = (
        p.get(spec, "emf"),
        p.get(spec, "internal"),
        p.get(spec, "load"),
    );
    let i = emf / (load + r);
    Board::new(spec, Analysis::Steady)
        .part("cell", (0.0, 2.0), (0.0, 0.0), cell(emf, r))
        .part("A", (0.0, 2.0), (2.0, 2.0), KindSpec::Ammeter)
        .wire(&[(2.0, 2.0), (4.0, 2.0)])
        .part("R", (4.0, 2.0), (4.0, 0.0), resistor(load))
        .wire(&[(4.0, 0.0), (0.0, 0.0)])
        .wire(&[(0.0, 2.0), (-1.5, 2.0)])
        .part("V", (-1.5, 2.0), (-1.5, 0.0), KindSpec::Voltmeter)
        .wire(&[(-1.5, 0.0), (0.0, 0.0)])
        .measured("Current, ε/(R + r) (A)", i, current("A"), 1.0)
        .measured(
            "Voltage across the terminals, ε − Ir (V)",
            emf - i * r,
            voltage("V"),
            1.0,
        )
        .measured(
            "Power in the resistance, I²R (W)",
            i * i * load,
            power("R"),
            1.0,
        )
        .answer(
            "Most power it can give any resistance, ε²/4r (W)",
            emf * emf / (4.0 * r),
        )
        .done()
}

static KIRCHHOFF: [Param; 5] = [
    param("left", "Left cell", "V", (0.0, 24.0), 0.5, 10.0),
    param("right", "Right cell", "V", (0.0, 24.0), 0.5, 4.0),
    param("first", "R₁", "Ω", (1.0, 20.0), 1.0, 2.0),
    param("second", "R₂", "Ω", (1.0, 20.0), 1.0, 1.0),
    param("third", "R₃, in the middle", "Ω", (1.0, 20.0), 1.0, 4.0),
];

fn kirchhoff(spec: &ScenarioSpec, p: &Params) -> CircuitScene {
    let (e1, e2) = (p.get(spec, "left"), p.get(spec, "right"));
    let (r1, r2, r3) = (
        p.get(spec, "first"),
        p.get(spec, "second"),
        p.get(spec, "third"),
    );
    // The top junction is at potential V above the bottom one: (ε₁ − V)/R₁ + (ε₂ − V)/R₂ = V/R₃.
    let v = (e1 / r1 + e2 / r2) / (1.0 / r1 + 1.0 / r2 + 1.0 / r3);
    Board::new(spec, Analysis::Steady)
        .part("left cell", (0.0, 2.0), (0.0, 0.0), cell(e1, 0.0))
        .part("R₁", (0.0, 2.0), (3.0, 2.0), resistor(r1))
        .part("R₃", (3.0, 2.0), (3.0, 0.0), resistor(r3))
        .part("R₂", (6.0, 2.0), (3.0, 2.0), resistor(r2))
        .part("right cell", (6.0, 2.0), (6.0, 0.0), cell(e2, 0.0))
        .wire(&[(6.0, 0.0), (3.0, 0.0), (0.0, 0.0)])
        .measured(
            "Current in R₁, towards the junction (A)",
            (e1 - v) / r1,
            current("R₁"),
            1.0,
        )
        .measured(
            "Current in R₂, towards the junction (A)",
            (e2 - v) / r2,
            current("R₂"),
            1.0,
        )
        .measured(
            "Current in R₃, away from it (A)",
            v / r3,
            current("R₃"),
            1.0,
        )
        .measured("Potential of the junction (V)", v, voltage("R₃"), 1.0)
        .done()
}

static BRIDGE: [Param; 5] = [
    param("p", "P", "Ω", (1.0, 100.0), 1.0, 10.0),
    param("q", "Q", "Ω", (1.0, 100.0), 1.0, 20.0),
    param("r", "R", "Ω", (1.0, 100.0), 1.0, 15.0),
    param("s", "S", "Ω", (1.0, 100.0), 1.0, 20.0),
    param("emf", "Battery", "V", (1.0, 12.0), 0.5, 6.0),
];

const GALVANOMETER: f64 = 50.0;

/// The current in the galvanometer of a bridge fed by an ideal battery, from the top junction to the bottom:
/// the open-circuit voltage between them over the resistance seen from the gap.
fn bridge_current(emf: f64, [p, q, r, s]: [f64; 4], meter: f64) -> f64 {
    let open = emf * (q / (p + q) - s / (r + s));
    open / (p * q / (p + q) + r * s / (r + s) + meter)
}

fn wheatstone(spec: &ScenarioSpec, p: &Params) -> CircuitScene {
    let arms = [
        p.get(spec, "p"),
        p.get(spec, "q"),
        p.get(spec, "r"),
        p.get(spec, "s"),
    ];
    let emf = p.get(spec, "emf");
    let [pp, q, r, _] = arms;
    let (left, top, right, bottom) = ((0.0, 2.0), (2.5, 4.0), (5.0, 2.0), (2.5, 0.0));
    Board::new(spec, Analysis::Steady)
        .part("battery", (0.0, -1.5), (5.0, -1.5), cell(emf, 0.0))
        .wire(&[(0.0, -1.5), left])
        .wire(&[(5.0, -1.5), right])
        .part("P", left, top, resistor(arms[0]))
        .part("Q", top, right, resistor(arms[1]))
        .part("R", left, bottom, resistor(arms[2]))
        .part("S", bottom, right, resistor(arms[3]))
        .part(
            "G",
            top,
            bottom,
            KindSpec::Galvanometer {
                resistance: GALVANOMETER,
            },
        )
        .measured(
            "Current in the galvanometer (mA)",
            bridge_current(emf, arms, GALVANOMETER) * MILLI,
            current("G"),
            MILLI,
        )
        .answer("The S that balances it, QR/P (Ω)", q * r / pp)
        .answer(
            "Balanced? (1 = yes)",
            f64::from((arms[0] * arms[3] - arms[1] * arms[2]).abs() < 1e-9),
        )
        .done()
}

static METER_BRIDGE: [Param; 3] = [
    param("known", "Known resistance, R", "Ω", (1.0, 20.0), 0.5, 4.0),
    param(
        "unknown",
        "Unknown resistance, S",
        "Ω",
        (1.0, 20.0),
        0.5,
        6.0,
    ),
    param(
        "jockey",
        "Jockey, from the left end",
        "cm",
        (5.0, 95.0),
        0.5,
        30.0,
    ),
];

/// Resistance of the metre of wire, and the battery across it.
const BRIDGE_WIRE: f64 = 5.0;
const BRIDGE_EMF: f64 = 2.0;

fn meter_bridge(spec: &ScenarioSpec, p: &Params) -> CircuitScene {
    let (known, unknown, at) = (
        p.get(spec, "known"),
        p.get(spec, "unknown"),
        p.get(spec, "jockey"),
    );
    let share = at / 100.0;
    // The page is ten units wide for the metre of wire.
    let (a, b, jockey) = ((0.0, 0.0), (10.0, 0.0), (10.0 * share, 0.0));
    let arms = [
        known,
        unknown,
        BRIDGE_WIRE * share,
        BRIDGE_WIRE * (1.0 - share),
    ];
    Board::new(spec, Analysis::Steady)
        .part("wire, left of the jockey", a, jockey, wire_of(arms[2]))
        .part("wire, right of the jockey", jockey, b, wire_of(arms[3]))
        .wire(&[a, (0.0, 3.0), (1.5, 3.0)])
        .part("R", (1.5, 3.0), (5.0, 3.0), resistor(known))
        .part("S", (5.0, 3.0), (8.5, 3.0), resistor(unknown))
        .wire(&[(8.5, 3.0), (10.0, 3.0), b])
        .part(
            "G",
            (5.0, 3.0),
            (5.0, 1.5),
            KindSpec::Galvanometer {
                resistance: GALVANOMETER,
            },
        )
        .wire(&[(5.0, 1.5), jockey])
        .wire(&[a, (0.0, -1.8), (3.5, -1.8)])
        .part("battery", (3.5, -1.8), (6.5, -1.8), cell(BRIDGE_EMF, 0.0))
        .wire(&[(6.5, -1.8), (10.0, -1.8), b])
        .measured(
            "Current in the galvanometer (mA)",
            bridge_current(BRIDGE_EMF, arms, GALVANOMETER) * MILLI,
            current("G"),
            MILLI,
        )
        .answer(
            "Balance point, 100R/(R + S) (cm)",
            100.0 * known / (known + unknown),
        )
        .answer(
            "S found from where the jockey is, R(100 − l)/l (Ω)",
            known * (100.0 - at) / at,
        )
        .done()
}

static POTENTIOMETER: [Param; 3] = [
    param("driver", "Driver cell", "V", (2.0, 12.0), 0.5, 4.0),
    param("test", "Cell being measured", "V", (0.5, 3.0), 0.05, 1.5),
    param(
        "jockey",
        "Jockey, from the left end",
        "cm",
        (2.0, 98.0),
        0.5,
        60.0,
    ),
];

const POT_WIRE: f64 = 10.0;
const TEST_INTERNAL: f64 = 1.0;

fn potentiometer(spec: &ScenarioSpec, p: &Params) -> CircuitScene {
    let (driver, test, at) = (
        p.get(spec, "driver"),
        p.get(spec, "test"),
        p.get(spec, "jockey"),
    );
    let share = at / 100.0;
    let (a, b, jockey) = ((0.0, 0.0), (10.0, 0.0), (10.0 * share, 0.0));
    let (left, right) = (POT_WIRE * share, POT_WIRE * (1.0 - share));
    // Seen from the jockey, the wire and driver are a cell of EMF V·l/L behind the two parts of the wire in
    // parallel. It is opposed by the cell being measured.
    let open = driver * share - test;
    let behind = left * right / POT_WIRE + TEST_INTERNAL + GALVANOMETER;
    Board::new(spec, Analysis::Steady)
        .part("wire, left of the jockey", a, jockey, wire_of(left))
        .part("wire, right of the jockey", jockey, b, wire_of(right))
        .wire(&[a, (0.0, 2.5), (3.5, 2.5)])
        .part("driver", (3.5, 2.5), (6.5, 2.5), cell(driver, 0.0))
        .wire(&[(6.5, 2.5), (10.0, 2.5), b])
        .wire(&[a, (0.0, -2.0), (1.5, -2.0)])
        .part(
            "test cell",
            (1.5, -2.0),
            (4.0, -2.0),
            cell(test, TEST_INTERNAL),
        )
        .part(
            "G",
            (4.0, -2.0),
            (10.0 * share, -2.0),
            KindSpec::Galvanometer {
                resistance: GALVANOMETER,
            },
        )
        .wire(&[(10.0 * share, -2.0), jockey])
        .measured(
            "Current in the galvanometer (mA)",
            open / behind * MILLI,
            current("G"),
            MILLI,
        )
        .answer("Balance point, 100ε/V (cm)", 100.0 * test / driver)
        .answer("Potential drop along the wire (V/cm)", driver / 100.0)
        .answer("EMF found from where the jockey is (V)", driver * share)
        .done()
}

static TWO_CELLS: [Param; 5] = [
    param("first", "EMF of the first cell", "V", (0.5, 6.0), 0.1, 1.5),
    param(
        "first_r",
        "Its internal resistance",
        "Ω",
        (0.1, 3.0),
        0.1,
        0.5,
    ),
    param(
        "second",
        "EMF of the second cell",
        "V",
        (0.5, 6.0),
        0.1,
        2.0,
    ),
    param(
        "second_r",
        "Its internal resistance",
        "Ω",
        (0.1, 3.0),
        0.1,
        1.0,
    ),
    param("load", "Resistance they feed", "Ω", (0.5, 20.0), 0.5, 3.0),
];

fn cells_in_parallel(spec: &ScenarioSpec, p: &Params) -> CircuitScene {
    let (e1, r1) = (p.get(spec, "first"), p.get(spec, "first_r"));
    let (e2, r2) = (p.get(spec, "second"), p.get(spec, "second_r"));
    let load = p.get(spec, "load");
    let together = (e1 * r2 + e2 * r1) / (r1 + r2);
    let inside = r1 * r2 / (r1 + r2);
    let i = together / (load + inside);
    let v = i * load;
    Board::new(spec, Analysis::Steady)
        .part("cell 1", (0.0, 2.0), (0.0, 0.0), cell(e1, r1))
        .part("cell 2", (2.0, 2.0), (2.0, 0.0), cell(e2, r2))
        .wire(&[(0.0, 2.0), (2.0, 2.0)])
        .part("A", (2.0, 2.0), (4.5, 2.0), KindSpec::Ammeter)
        .part("R", (4.5, 2.0), (4.5, 0.0), resistor(load))
        .wire(&[(4.5, 0.0), (2.0, 0.0), (0.0, 0.0)])
        .answer("EMF of the pair, (ε₁r₂ + ε₂r₁)/(r₁ + r₂) (V)", together)
        .answer("Internal resistance of the pair (Ω)", inside)
        .measured("Current in the resistance (A)", i, current("A"), 1.0)
        .measured(
            "Current supplied by cell 1 (A)",
            (e1 - v) / r1,
            current("cell 1"),
            -1.0,
        )
        .measured(
            "Current supplied by cell 2 (A)",
            (e2 - v) / r2,
            current("cell 2"),
            -1.0,
        )
        .done()
}

// ---------------------------------------------------------------- capacitors

/// A circuit followed for `duration`, in two thousand steps, and played back over about eight seconds.
fn followed(duration: f64) -> Analysis {
    Analysis::Transient {
        dt: duration / 2000.0,
        duration,
        playback: duration / 8.0,
    }
}

fn capacitor(capacitance: f64, initial_voltage: f64) -> KindSpec {
    KindSpec::Capacitor {
        capacitance,
        initial_voltage,
    }
}

static CHARGING: [Param; 3] = [
    param("emf", "Battery", "V", (1.0, 24.0), 0.5, 9.0),
    param("resistance", "Resistance", "kΩ", (1.0, 100.0), 1.0, 10.0),
    param("capacitance", "Capacitance", "μF", (1.0, 500.0), 1.0, 100.0),
];

fn charging(spec: &ScenarioSpec, p: &Params) -> CircuitScene {
    let emf = p.get(spec, "emf");
    let (r, c) = (
        p.get(spec, "resistance") * 1e3,
        p.get(spec, "capacitance") * 1e-6,
    );
    let tau = r * c;
    let level = emf * (1.0 - (-1.0f64).exp());
    Board::new(spec, followed(6.0 * tau))
        .part("battery", (0.0, 2.0), (0.0, 0.0), cell(emf, 0.0))
        .part(
            "switch",
            (0.0, 2.0),
            (1.5, 2.0),
            KindSpec::Switch {
                closed: true,
                flips_at: None,
            },
        )
        .part("R", (1.5, 2.0), (4.0, 2.0), resistor(r))
        .part("C", (4.0, 2.0), (4.0, 0.0), capacitor(c, 0.0))
        .wire(&[(4.0, 0.0), (0.0, 0.0)])
        .watch(&["C", "R"])
        .answer("Time constant, RC (s)", tau)
        .measured(
            "Time to reach 63.2 % of the battery's voltage (s)",
            tau,
            MeasureSpec::TimeToReach {
                element: "C".to_string(),
                quantity: Quantity::Voltage,
                value: level,
            },
            1.0,
        )
        .measured(
            "Current at the start, V/R (mA)",
            emf / r * MILLI,
            MeasureSpec::Peak {
                element: "R".to_string(),
                quantity: Quantity::Current,
            },
            MILLI,
        )
        .measured(
            "Voltage across the capacitor at the end (V)",
            emf * (1.0 - (-6.0f64).exp()),
            voltage("C"),
            1.0,
        )
        .answer("Charge when full, CV (μC)", c * emf * MICRO)
        .answer(
            "Energy stored when full, CV²/2 (mJ)",
            0.5 * c * emf * emf * MILLI,
        )
        .done()
}

static DISCHARGING: [Param; 3] = [
    param(
        "voltage",
        "Voltage it starts at",
        "V",
        (1.0, 24.0),
        0.5,
        9.0,
    ),
    param("resistance", "Resistance", "kΩ", (1.0, 100.0), 1.0, 10.0),
    param("capacitance", "Capacitance", "μF", (1.0, 500.0), 1.0, 100.0),
];

fn discharging(spec: &ScenarioSpec, p: &Params) -> CircuitScene {
    let v0 = p.get(spec, "voltage");
    let (r, c) = (
        p.get(spec, "resistance") * 1e3,
        p.get(spec, "capacitance") * 1e-6,
    );
    let tau = r * c;
    Board::new(spec, followed(6.0 * tau))
        .part("C", (0.0, 2.0), (0.0, 0.0), capacitor(c, v0))
        .part(
            "switch",
            (0.0, 2.0),
            (1.5, 2.0),
            KindSpec::Switch {
                closed: true,
                flips_at: None,
            },
        )
        .wire(&[(1.5, 2.0), (4.0, 2.0)])
        .part("R", (4.0, 2.0), (4.0, 0.0), resistor(r))
        .wire(&[(4.0, 0.0), (0.0, 0.0)])
        .watch(&["C", "R"])
        .answer("Time constant, RC (s)", tau)
        .measured(
            "Time to fall to 36.8 % (s)",
            tau,
            MeasureSpec::TimeToReach {
                element: "C".to_string(),
                quantity: Quantity::Voltage,
                value: v0 * (-1.0f64).exp(),
            },
            1.0,
        )
        .measured(
            "Time to fall to half, RC ln 2 (s)",
            tau * 2f64.ln(),
            MeasureSpec::TimeToReach {
                element: "C".to_string(),
                quantity: Quantity::Voltage,
                value: 0.5 * v0,
            },
            1.0,
        )
        .measured(
            "Current at the start, V₀/R (mA)",
            v0 / r * MILLI,
            MeasureSpec::Peak {
                element: "R".to_string(),
                quantity: Quantity::Current,
            },
            MILLI,
        )
        .answer(
            "Energy turned to heat, CV₀²/2 (mJ)",
            0.5 * c * v0 * v0 * MILLI,
        )
        .done()
}

static TWO_CAPACITORS: [Param; 3] = [
    param("emf", "Battery", "V", (1.0, 24.0), 0.5, 12.0),
    param("first", "C₁", "μF", (1.0, 100.0), 1.0, 20.0),
    param("second", "C₂", "μF", (1.0, 100.0), 1.0, 30.0),
];

/// Charging resistance: something has to limit the current, or the capacitors would charge in no time.
const CHARGING_RESISTANCE: f64 = 1000.0;

fn charge_of(element: &str) -> MeasureSpec {
    MeasureSpec::Charge {
        element: element.to_string(),
    }
}

fn capacitors_in_series(spec: &ScenarioSpec, p: &Params) -> CircuitScene {
    let emf = p.get(spec, "emf");
    let (c1, c2) = (p.get(spec, "first") * 1e-6, p.get(spec, "second") * 1e-6);
    let together = c1 * c2 / (c1 + c2);
    let q = together * emf;
    Board::new(spec, followed(12.0 * CHARGING_RESISTANCE * together))
        .part("battery", (0.0, 2.0), (0.0, 0.0), cell(emf, 0.0))
        .part("R", (0.0, 2.0), (2.0, 2.0), resistor(CHARGING_RESISTANCE))
        .part("C₁", (2.0, 2.0), (4.0, 2.0), capacitor(c1, 0.0))
        .wire(&[(4.0, 2.0), (5.0, 2.0)])
        .part("C₂", (5.0, 2.0), (5.0, 0.0), capacitor(c2, 0.0))
        .wire(&[(5.0, 0.0), (0.0, 0.0)])
        .watch(&["C₁", "C₂"])
        .answer(
            "Equivalent capacitance, C₁C₂/(C₁ + C₂) (μF)",
            together * MICRO,
        )
        .measured("Charge on C₁ (μC)", q * MICRO, charge_of("C₁"), MICRO)
        .measured(
            "Charge on C₂: the same (μC)",
            q * MICRO,
            charge_of("C₂"),
            MICRO,
        )
        .measured("Voltage across C₁ (V)", q / c1, voltage("C₁"), 1.0)
        .measured("Voltage across C₂ (V)", q / c2, voltage("C₂"), 1.0)
        .done()
}

fn capacitors_in_parallel(spec: &ScenarioSpec, p: &Params) -> CircuitScene {
    let emf = p.get(spec, "emf");
    let (c1, c2) = (p.get(spec, "first") * 1e-6, p.get(spec, "second") * 1e-6);
    Board::new(spec, followed(12.0 * CHARGING_RESISTANCE * (c1 + c2)))
        .part("battery", (0.0, 2.0), (0.0, 0.0), cell(emf, 0.0))
        .part("R", (0.0, 2.0), (2.5, 2.0), resistor(CHARGING_RESISTANCE))
        .wire(&[(2.5, 2.0), (4.5, 2.0)])
        .part("C₁", (2.5, 2.0), (2.5, 0.0), capacitor(c1, 0.0))
        .part("C₂", (4.5, 2.0), (4.5, 0.0), capacitor(c2, 0.0))
        .wire(&[(4.5, 0.0), (2.5, 0.0), (0.0, 0.0)])
        .watch(&["C₁", "C₂"])
        .answer("Equivalent capacitance, C₁ + C₂ (μF)", (c1 + c2) * MICRO)
        .measured("Voltage across each (V)", emf, voltage("C₁"), 1.0)
        .measured(
            "Charge on C₁ (μC)",
            c1 * emf * MICRO,
            charge_of("C₁"),
            MICRO,
        )
        .measured(
            "Charge on C₂ (μC)",
            c2 * emf * MICRO,
            charge_of("C₂"),
            MICRO,
        )
        .answer(
            "Energy stored in the pair (mJ)",
            0.5 * (c1 + c2) * emf * emf * MILLI,
        )
        .done()
}

// ---------------------------------------------------------------- inductors and alternating current

static LR: [Param; 3] = [
    param("emf", "Battery", "V", (1.0, 24.0), 0.5, 6.0),
    param("resistance", "Resistance", "Ω", (1.0, 100.0), 1.0, 10.0),
    param("inductance", "Inductance", "H", (0.1, 10.0), 0.1, 2.0),
];

fn lr_circuit(spec: &ScenarioSpec, p: &Params) -> CircuitScene {
    let (emf, r, l) = (
        p.get(spec, "emf"),
        p.get(spec, "resistance"),
        p.get(spec, "inductance"),
    );
    let tau = l / r;
    let last = emf / r;
    Board::new(spec, followed(6.0 * tau))
        .part("battery", (0.0, 2.0), (0.0, 0.0), cell(emf, 0.0))
        .part(
            "switch",
            (0.0, 2.0),
            (1.5, 2.0),
            KindSpec::Switch {
                closed: true,
                flips_at: None,
            },
        )
        .part("R", (1.5, 2.0), (4.0, 2.0), resistor(r))
        .part(
            "L",
            (4.0, 2.0),
            (4.0, 0.0),
            KindSpec::Inductor {
                inductance: l,
                initial_current: 0.0,
            },
        )
        .wire(&[(4.0, 0.0), (0.0, 0.0)])
        .watch(&["L", "R"])
        .answer("Time constant, L/R (s)", tau)
        .measured(
            "Time to reach 63.2 % of the final current (s)",
            tau,
            MeasureSpec::TimeToReach {
                element: "L".to_string(),
                quantity: Quantity::Current,
                value: last * (1.0 - (-1.0f64).exp()),
            },
            1.0,
        )
        .measured(
            "Back EMF at the start: the whole battery (V)",
            emf,
            MeasureSpec::Peak {
                element: "L".to_string(),
                quantity: Quantity::Voltage,
            },
            1.0,
        )
        .measured(
            "Current at the end (A)",
            last * (1.0 - (-6.0f64).exp()),
            current("L"),
            1.0,
        )
        .answer("Final current, V/R (A)", last)
        .answer("Energy stored in the end, LI²/2 (J)", 0.5 * l * last * last)
        .done()
}

static LC: [Param; 3] = [
    param(
        "voltage",
        "Voltage the capacitor starts at",
        "V",
        (1.0, 24.0),
        0.5,
        10.0,
    ),
    param("inductance", "Inductance", "mH", (1.0, 500.0), 1.0, 100.0),
    param("capacitance", "Capacitance", "μF", (1.0, 500.0), 1.0, 100.0),
];

fn lc_oscillations(spec: &ScenarioSpec, p: &Params) -> CircuitScene {
    let v0 = p.get(spec, "voltage");
    let (l, c) = (
        p.get(spec, "inductance") * 1e-3,
        p.get(spec, "capacitance") * 1e-6,
    );
    let period = TAU * (l * c).sqrt();
    let duration = 3.25 * period;
    let analysis = Analysis::Transient {
        dt: period / 2000.0,
        duration,
        playback: duration / 10.0,
    };
    Board::new(spec, analysis)
        .part("C", (0.0, 2.0), (0.0, 0.0), capacitor(c, v0))
        .wire(&[(0.0, 2.0), (4.0, 2.0)])
        .part(
            "L",
            (4.0, 2.0),
            (4.0, 0.0),
            KindSpec::Inductor {
                inductance: l,
                initial_current: 0.0,
            },
        )
        .wire(&[(4.0, 0.0), (0.0, 0.0)])
        .watch(&["C", "L"])
        .measured(
            "Period, 2π√(LC) (ms)",
            period * MILLI,
            MeasureSpec::Period {
                element: "L".to_string(),
            },
            MILLI,
        )
        .answer("Frequency (Hz)", 1.0 / period)
        .measured(
            "Greatest current, V₀√(C/L) (A)",
            v0 * (c / l).sqrt(),
            MeasureSpec::Peak {
                element: "L".to_string(),
                quantity: Quantity::Current,
            },
            1.0,
        )
        .answer(
            "Energy passed back and forth, CV₀²/2 (mJ)",
            0.5 * c * v0 * v0 * MILLI,
        )
        .done()
}

static AC_RESISTOR: [Param; 3] = [
    param(
        "amplitude",
        "Peak voltage of the source",
        "V",
        (1.0, 400.0),
        1.0,
        311.0,
    ),
    param("frequency", "Frequency", "Hz", (10.0, 200.0), 1.0, 50.0),
    param("resistance", "Resistance", "Ω", (10.0, 1000.0), 10.0, 100.0),
];

static AC_INDUCTOR: [Param; 3] = [
    param(
        "amplitude",
        "Peak voltage of the source",
        "V",
        (1.0, 400.0),
        1.0,
        311.0,
    ),
    param("frequency", "Frequency", "Hz", (10.0, 200.0), 1.0, 50.0),
    param(
        "inductance",
        "Inductance",
        "mH",
        (10.0, 1000.0),
        10.0,
        250.0,
    ),
];

static AC_CAPACITOR: [Param; 3] = [
    param(
        "amplitude",
        "Peak voltage of the source",
        "V",
        (1.0, 400.0),
        1.0,
        311.0,
    ),
    param("frequency", "Frequency", "Hz", (10.0, 200.0), 1.0, 50.0),
    param("capacitance", "Capacitance", "μF", (1.0, 200.0), 1.0, 15.0),
];

/// One cycle takes about four seconds to watch.
fn alternating(frequency: f64) -> Analysis {
    Analysis::Alternating {
        playback: 0.25 / frequency,
    }
}

/// A source across one component, with an ammeter in the loop. `opposition` is its resistance or reactance.
fn one_on_ac(
    spec: &ScenarioSpec,
    p: &Params,
    name: &str,
    kind: KindSpec,
    opposition: f64,
    lag: f64,
) -> Board {
    let (amplitude, frequency) = (p.get(spec, "amplitude"), p.get(spec, "frequency"));
    let source = KindSpec::Source {
        amplitude,
        frequency,
        phase: 0.0,
    };
    let peak = amplitude / opposition;
    Board::new(spec, alternating(frequency))
        .part("source", (0.0, 2.0), (0.0, 0.0), source)
        .part("A", (0.0, 2.0), (2.0, 2.0), KindSpec::Ammeter)
        .wire(&[(2.0, 2.0), (4.0, 2.0)])
        .part(name, (4.0, 2.0), (4.0, 0.0), kind)
        .wire(&[(4.0, 0.0), (0.0, 0.0)])
        .watch(&["source", "A"])
        .measured(
            "Peak current (A)",
            peak,
            MeasureSpec::Amplitude {
                element: "A".to_string(),
                quantity: Quantity::Current,
            },
            1.0,
        )
        .answer("RMS voltage, V₀/√2 (V)", amplitude / 2f64.sqrt())
        .answer("RMS current, I₀/√2 (A)", peak / 2f64.sqrt())
        .measured(
            "Current behind the voltage by (°)",
            lag,
            MeasureSpec::CurrentLag {
                current: "A".to_string(),
                voltage: "source".to_string(),
            },
            1.0,
        )
        .measured(
            "Average power taken (W)",
            0.5 * amplitude * peak * lag.to_radians().cos(),
            MeasureSpec::MeanPower {
                element: name.to_string(),
            },
            1.0,
        )
}

fn ac_resistor(spec: &ScenarioSpec, p: &Params) -> CircuitScene {
    let r = p.get(spec, "resistance");
    one_on_ac(spec, p, "R", resistor(r), r, 0.0).done()
}

fn ac_inductor(spec: &ScenarioSpec, p: &Params) -> CircuitScene {
    let l = p.get(spec, "inductance") * 1e-3;
    let reactance = TAU * p.get(spec, "frequency") * l;
    let kind = KindSpec::Inductor {
        inductance: l,
        initial_current: 0.0,
    };
    one_on_ac(spec, p, "L", kind, reactance, 90.0)
        .answer("Inductive reactance, 2πfL (Ω)", reactance)
        .done()
}

fn ac_capacitor(spec: &ScenarioSpec, p: &Params) -> CircuitScene {
    let c = p.get(spec, "capacitance") * 1e-6;
    let reactance = 1.0 / (TAU * p.get(spec, "frequency") * c);
    one_on_ac(spec, p, "C", capacitor(c, 0.0), reactance, -90.0)
        .answer("Capacitive reactance, 1/2πfC (Ω)", reactance)
        .done()
}

static LCR: [Param; 5] = [
    param(
        "amplitude",
        "Peak voltage of the source",
        "V",
        (1.0, 400.0),
        1.0,
        100.0,
    ),
    param("frequency", "Frequency", "Hz", (5.0, 500.0), 1.0, 50.0),
    param("resistance", "Resistance", "Ω", (1.0, 200.0), 1.0, 20.0),
    param(
        "inductance",
        "Inductance",
        "mH",
        (10.0, 1000.0),
        10.0,
        200.0,
    ),
    param("capacitance", "Capacitance", "μF", (1.0, 500.0), 1.0, 100.0),
];

fn series_lcr(spec: &ScenarioSpec, p: &Params) -> CircuitScene {
    let (amplitude, frequency) = (p.get(spec, "amplitude"), p.get(spec, "frequency"));
    let r = p.get(spec, "resistance");
    let (l, c) = (
        p.get(spec, "inductance") * 1e-3,
        p.get(spec, "capacitance") * 1e-6,
    );
    let omega = TAU * frequency;
    let (xl, xc) = (omega * l, 1.0 / (omega * c));
    let z = r.hypot(xl - xc);
    let peak = amplitude / z;
    let lag = (xl - xc).atan2(r);
    let amplitude_of = |element: &str, quantity| MeasureSpec::Amplitude {
        element: element.to_string(),
        quantity,
    };
    Board::new(spec, alternating(frequency))
        .part(
            "source",
            (0.0, 2.0),
            (0.0, 0.0),
            KindSpec::Source {
                amplitude,
                frequency,
                phase: 0.0,
            },
        )
        .part("A", (0.0, 2.0), (1.5, 2.0), KindSpec::Ammeter)
        .part("R", (1.5, 2.0), (3.5, 2.0), resistor(r))
        .part(
            "L",
            (3.5, 2.0),
            (5.5, 2.0),
            KindSpec::Inductor {
                inductance: l,
                initial_current: 0.0,
            },
        )
        .wire(&[(5.5, 2.0), (6.5, 2.0)])
        .part("C", (6.5, 2.0), (6.5, 0.0), capacitor(c, 0.0))
        .wire(&[(6.5, 0.0), (0.0, 0.0)])
        .watch(&["source", "A"])
        .measured(
            "Impedance, √(R² + (X_L − X_C)²) (Ω)",
            z,
            MeasureSpec::Impedance {
                voltage: "source".to_string(),
                current: "A".to_string(),
            },
            1.0,
        )
        .measured(
            "Peak current, V₀/Z (A)",
            peak,
            amplitude_of("A", Quantity::Current),
            1.0,
        )
        .measured(
            "Current behind the voltage by (°)",
            lag.to_degrees(),
            MeasureSpec::CurrentLag {
                current: "A".to_string(),
                voltage: "source".to_string(),
            },
            1.0,
        )
        .answer(
            "Resonant frequency, 1/2π√(LC) (Hz)",
            1.0 / (TAU * (l * c).sqrt()),
        )
        .measured(
            "Peak voltage across the inductor (V)",
            peak * xl,
            amplitude_of("L", Quantity::Voltage),
            1.0,
        )
        .measured(
            "Peak voltage across the capacitor (V)",
            peak * xc,
            amplitude_of("C", Quantity::Voltage),
            1.0,
        )
        .answer("Power factor, cos φ", lag.cos())
        .done()
}

// ---------------------------------------------------------------- catalogue

const CURRENT: &str = "Current electricity";
const CAPACITANCE: &str = "Electrostatic potential and capacitance";
const INDUCTION: &str = "Electromagnetic induction";
const AC: &str = "Alternating current";

const fn spec(
    id: &'static str,
    title: &'static str,
    description: &'static str,
    chapter: &'static str,
    topic: &'static str,
    params: &'static [Param],
) -> ScenarioSpec {
    ScenarioSpec {
        id,
        title,
        description,
        syllabus: Syllabus {
            class: 12,
            chapter,
            topic,
        },
        params,
    }
}

#[rustfmt::skip]
static CATALOGUE: &[Scenario] = &[
    Scenario {
        spec: spec("ohms-law", "Ohm's law", "A battery drives a current through a resistor. Double the voltage and the current doubles; double the resistance and it halves. The ammeter is in the path of the current, the voltmeter across the resistor.", CURRENT, "Ohm's law", &OHM),
        describe: ohms_law,
    },
    Scenario {
        spec: spec("resistors-in-series", "Resistors in series", "One path, so the same current goes through all three. The battery's voltage is shared out between them, the largest resistance taking the largest share.", CURRENT, "combination of resistors", &THREE),
        describe: series,
    },
    Scenario {
        spec: spec("resistors-in-parallel", "Resistors in parallel", "Three paths side by side, each with the whole of the battery's voltage across it. The current divides, most of it taking the easiest path, and together they pass more than any one alone.", CURRENT, "combination of resistors", &THREE),
        describe: parallel,
    },
    Scenario {
        spec: spec("internal-resistance", "A cell with internal resistance", "A real cell has some resistance inside it. The more current it gives, the more of its EMF is lost on the way out, and the voltage at its terminals sags. It gives the most power to a resistance equal to its own.", CURRENT, "cells, EMF and internal resistance", &INTERNAL),
        describe: internal_resistance,
    },
    Scenario {
        spec: spec("cells-in-parallel", "Two cells in parallel", "Two different cells side by side, feeding one resistance. They act as a single cell whose EMF lies between the two. If one is much weaker, the stronger drives current backwards through it.", CURRENT, "cells in series and in parallel", &TWO_CELLS),
        describe: cells_in_parallel,
    },
    Scenario {
        spec: spec("kirchhoff-rules", "Kirchhoff's rules", "Two cells and three resistors, and no way to reduce them to series and parallel. Two rules are enough: what flows into a junction flows out, and round any closed path the rises and falls of potential add to zero.", CURRENT, "Kirchhoff's rules", &KIRCHHOFF),
        describe: kirchhoff,
    },
    Scenario {
        spec: spec("wheatstone-bridge", "The Wheatstone bridge", "Four resistors in a diamond with a galvanometer across the middle. When P/Q = R/S the two ends of the galvanometer are at the same potential and no current flows through it, whatever the battery. Change S until the reading is zero.", CURRENT, "Wheatstone bridge", &BRIDGE),
        describe: wheatstone,
    },
    Scenario {
        spec: spec("meter-bridge", "The metre bridge", "A Wheatstone bridge in which two of the arms are the two parts of a metre of wire. Slide the jockey along the wire until the galvanometer reads zero; the lengths either side then give the unknown resistance.", CURRENT, "metre bridge", &METER_BRIDGE),
        describe: meter_bridge,
    },
    Scenario {
        spec: spec("potentiometer", "The potentiometer", "A steady current in a long wire makes the potential fall evenly along it. Slide the jockey until the fall from the end matches the cell being measured: the galvanometer reads zero, the cell gives no current, and so its true EMF is measured.", CURRENT, "potentiometer", &POTENTIOMETER),
        describe: potentiometer,
    },
    Scenario {
        spec: spec("charging-capacitor", "Charging a capacitor", "At first the capacitor is empty and the whole of the battery's voltage drives current through the resistor. As charge builds up the capacitor pushes back, the current falls, and the charging slows: fast at first, then ever more gently.", CAPACITANCE, "charging through a resistance", &CHARGING),
        describe: charging,
    },
    Scenario {
        spec: spec("discharging-capacitor", "Discharging a capacitor", "A charged capacitor emptying through a resistor. The current is greatest at the start and dies away as the voltage does. In every interval RC the voltage falls by the same fraction.", CAPACITANCE, "discharging through a resistance", &DISCHARGING),
        describe: discharging,
    },
    Scenario {
        spec: spec("capacitors-in-series", "Capacitors in series", "The charge that leaves one plate of C₁ has nowhere to go but the facing plate of C₂, so both end up with the same charge. The smaller capacitor takes the larger share of the voltage.", CAPACITANCE, "combination of capacitors", &TWO_CAPACITORS),
        describe: capacitors_in_series,
    },
    Scenario {
        spec: spec("capacitors-in-parallel", "Capacitors in parallel", "Both capacitors have the battery's voltage across them, and each takes charge in proportion to its capacitance. Together they hold as much as one capacitor of their sum.", CAPACITANCE, "combination of capacitors", &TWO_CAPACITORS),
        describe: capacitors_in_parallel,
    },
    Scenario {
        spec: spec("lr-circuit", "Current growing in an inductor", "An inductor opposes any change in the current through it. Switch on, and at first it holds the current at zero by setting up a back EMF equal to the battery. The current then grows, quickly at first and then more slowly, towards V/R.", INDUCTION, "self-inductance", &LR),
        describe: lr_circuit,
    },
    Scenario {
        spec: spec("lc-oscillations", "LC oscillations", "A charged capacitor joined to an inductor. The capacitor empties through the coil, the coil keeps the current going and charges it up the other way, and back it comes: energy swinging between electric and magnetic like a mass on a spring.", AC, "LC oscillations", &LC),
        describe: lc_oscillations,
    },
    Scenario {
        spec: spec("ac-resistor", "A resistor on alternating current", "The current follows the voltage exactly, rising and falling in step with it. The power is always being turned to heat, on average half of its peak.", AC, "AC applied to a resistor", &AC_RESISTOR),
        describe: ac_resistor,
    },
    Scenario {
        spec: spec("ac-inductor", "An inductor on alternating current", "The inductor's back EMF is largest when the current is changing fastest, which is as it passes through zero. So the current reaches its peak a quarter of a cycle after the voltage. Energy goes in and comes out again; none is used up.", AC, "AC applied to an inductor", &AC_INDUCTOR),
        describe: ac_inductor,
    },
    Scenario {
        spec: spec("ac-capacitor", "A capacitor on alternating current", "Current flows while the capacitor is charging, and is largest when the voltage is changing fastest. So the current reaches its peak a quarter of a cycle before the voltage. The higher the frequency, the more easily it passes.", AC, "AC applied to a capacitor", &AC_CAPACITOR),
        describe: ac_capacitor,
    },
    Scenario {
        spec: spec("series-lcr", "The series LCR circuit", "The inductor and the capacitor pull the phase opposite ways, and at one frequency they cancel exactly. There the circuit is a plain resistance, the current is at its largest, and the voltages across L and C can each be far bigger than the source. That is resonance, and how a radio is tuned.", AC, "series LCR circuit and resonance", &LCR),
        describe: series_lcr,
    },
];
