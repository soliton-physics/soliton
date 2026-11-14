//! A circuit as data: junctions with places on the page, the components between them, how it is to be solved,
//! and the textbook answers with how each is read off the solution. SI units throughout.

use crate::circuit::{Circuit, Kind, Phasor, Reading, State};
use serde::{Deserialize, Serialize};
use std::f64::consts::TAU;

pub const SCHEMA: u32 = 1;

#[derive(Debug, Clone, PartialEq)]
pub struct SceneError(pub String);

impl std::fmt::Display for SceneError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for SceneError {}

fn err<T>(message: impl Into<String>) -> Result<T, SceneError> {
    Err(SceneError(message.into()))
}

fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

fn is_false(v: &bool) -> bool {
    !*v
}

/// A component, in the words of [`Kind`].
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KindSpec {
    Wire,
    Resistor {
        resistance: f64,
        /// A length of resistance wire, to be drawn as a wire and not as a resistor.
        #[serde(default, skip_serializing_if = "is_false")]
        length_of_wire: bool,
    },
    /// Positive terminal at `a`.
    Battery {
        emf: f64,
        #[serde(default, skip_serializing_if = "is_zero")]
        internal: f64,
    },
    Capacitor {
        capacitance: f64,
        #[serde(default, skip_serializing_if = "is_zero")]
        initial_voltage: f64,
    },
    Inductor {
        inductance: f64,
        #[serde(default, skip_serializing_if = "is_zero")]
        initial_current: f64,
    },
    /// `amplitude · sin(2π · frequency · t + phase)`.
    Source {
        amplitude: f64,
        frequency: f64,
        #[serde(default, skip_serializing_if = "is_zero")]
        phase: f64,
    },
    Switch {
        #[serde(default, skip_serializing_if = "is_false")]
        closed: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        flips_at: Option<f64>,
    },
    Ammeter,
    Voltmeter,
    Galvanometer {
        resistance: f64,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ElementSpec {
    pub name: String,
    /// Junctions it joins, by number. Its voltage is that of `a` above `b`; its current runs from `a` to `b`.
    pub a: usize,
    pub b: usize,
    pub kind: KindSpec,
}

/// How the circuit is to be solved and shown.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Analysis {
    /// Steady currents: solved once.
    Steady,
    /// Followed in time from the moment it is switched on, in steps of `dt` for `duration` seconds, and shown
    /// at `playback` seconds of the circuit to one of watching.
    Transient {
        dt: f64,
        duration: f64,
        playback: f64,
    },
    /// The steady alternating state, shown at `playback` seconds of the circuit to one of watching.
    Alternating { playback: f64 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Quantity {
    Voltage,
    Current,
}

/// How an answer is read off the solved circuit. Each result is multiplied by the answer's `scale`, so that an
/// answer in milliamperes has a scale of 1000.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeasureSpec {
    /// Through a component now, from its `a` to its `b`.
    Current { element: String },
    /// Across a component now.
    Voltage { element: String },
    /// Taken by a component now, W.
    Power { element: String },
    /// On a capacitor now, C.
    Charge { element: String },
    /// Stored in a capacitor or an inductor now, J.
    Energy { element: String },
    /// When a quantity first reaches `value`, s.
    TimeToReach {
        element: String,
        quantity: Quantity,
        value: f64,
    },
    /// The largest a quantity has been so far, without sign.
    Peak { element: String, quantity: Quantity },
    /// The time between successive moments a current passes through zero going the same way, s.
    Period { element: String },
    /// The peak value of an alternating quantity.
    Amplitude { element: String, quantity: Quantity },
    /// By how many degrees the current in one component is behind the voltage across another.
    CurrentLag { current: String, voltage: String },
    /// The amplitude of the voltage across one component over that of the current in another, Ω.
    Impedance { voltage: String, current: String },
    /// Taken by a component averaged over a cycle, W.
    MeanPower { element: String },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnswerSpec {
    pub name: String,
    pub value: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub measure: Option<MeasureSpec>,
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub scale: f64,
}

fn one() -> f64 {
    1.0
}

fn is_one(v: &f64) -> bool {
    *v == 1.0
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SyllabusSpec {
    pub class: u8,
    pub chapter: String,
    pub topic: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CircuitScene {
    pub schema: u32,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub syllabus: Option<SyllabusSpec>,
    /// Where each junction is drawn. The first is the one potential is measured from.
    pub nodes: Vec<[f64; 2]>,
    pub elements: Vec<ElementSpec>,
    pub analysis: Analysis,
    /// The components whose voltage and current are worth graphing.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub watch: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub answers: Vec<AnswerSpec>,
}

/// A scene solved, and for one that changes, being followed in time.
#[derive(Clone, Debug)]
pub struct Bench {
    pub scene: CircuitScene,
    pub circuit: Circuit,
    /// The circuit now.
    pub state: State,
    /// Every step so far of a circuit followed in time.
    pub history: Vec<State>,
    /// The alternating amplitudes, for a circuit solved that way.
    pub phasors: Option<Vec<Phasor>>,
}

impl CircuitScene {
    pub fn from_json(text: &str) -> Result<Self, SceneError> {
        let scene: Self = serde_json::from_str(text)
            .map_err(|e| SceneError(format!("invalid scene JSON: {e}")))?;
        scene.start()?;
        Ok(scene)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("a circuit scene always serialises")
    }

    fn circuit(&self) -> Result<Circuit, SceneError> {
        if self.schema != SCHEMA {
            return err(format!("unsupported scene schema {}", self.schema));
        }
        if self.nodes.is_empty() {
            return err("a circuit needs at least one junction");
        }
        let mut circuit = Circuit::new(self.nodes.len());
        for element in &self.elements {
            let name = &element.name;
            if element.a >= self.nodes.len() || element.b >= self.nodes.len() {
                return err(format!("{name}: joins a junction that is not there"));
            }
            if element.a == element.b {
                return err(format!("{name}: both ends are at the same junction"));
            }
            let positive = |value: f64, what: &str| {
                if value.is_nan() || value <= 0.0 {
                    err(format!("{name}: {what} must be positive"))
                } else {
                    Ok(value)
                }
            };
            let kind = match element.kind {
                KindSpec::Wire => Kind::Wire,
                KindSpec::Ammeter => Kind::Ammeter,
                KindSpec::Voltmeter => Kind::Voltmeter,
                KindSpec::Resistor { resistance, .. } => Kind::Resistor {
                    resistance: positive(resistance, "resistance")?,
                },
                KindSpec::Galvanometer { resistance } => Kind::Galvanometer {
                    resistance: positive(resistance, "resistance")?,
                },
                KindSpec::Battery { emf, internal } => {
                    if !emf.is_finite() || internal.is_nan() || internal < 0.0 {
                        return err(format!("{name}: a cell needs an EMF and an internal resistance of zero or more"));
                    }
                    Kind::Battery { emf, internal }
                }
                KindSpec::Capacitor {
                    capacitance,
                    initial_voltage,
                } => Kind::Capacitor {
                    capacitance: positive(capacitance, "capacitance")?,
                    initial_voltage,
                },
                KindSpec::Inductor {
                    inductance,
                    initial_current,
                } => Kind::Inductor {
                    inductance: positive(inductance, "inductance")?,
                    initial_current,
                },
                KindSpec::Source {
                    amplitude,
                    frequency,
                    phase,
                } => Kind::Source {
                    amplitude,
                    frequency: positive(frequency, "frequency")?,
                    phase,
                },
                KindSpec::Switch { closed, flips_at } => Kind::Switch { closed, flips_at },
            };
            circuit.add(name, element.a, element.b, kind);
        }
        match self.analysis {
            Analysis::Steady => {}
            Analysis::Transient {
                dt,
                duration,
                playback,
            } => {
                if [dt, duration, playback]
                    .iter()
                    .any(|v| v.is_nan() || *v <= 0.0)
                {
                    return err("transient: dt, duration and playback must be positive");
                }
            }
            Analysis::Alternating { playback } => {
                if playback.is_nan() || playback <= 0.0 {
                    return err("alternating: playback must be positive");
                }
                if circuit.frequency().is_none() {
                    return err("alternating: there is no alternating source");
                }
            }
        }
        Ok(circuit)
    }

    /// Solves the circuit as it is at the start.
    pub fn start(&self) -> Result<Bench, SceneError> {
        let circuit = self.circuit()?;
        let stuck = || {
            SceneError("Kirchhoff's laws do not settle this circuit: is there a loop of ideal cells and wires, or two capacitors joined with no resistance between them?".to_string())
        };
        let mut bench = Bench {
            scene: self.clone(),
            circuit,
            state: State::default(),
            history: Vec::new(),
            phasors: None,
        };
        match self.analysis {
            Analysis::Steady => bench.state = bench.circuit.steady().ok_or_else(stuck)?,
            Analysis::Transient { .. } => {
                bench.state = bench.circuit.start().ok_or_else(stuck)?;
                bench.history.push(bench.state.clone());
            }
            Analysis::Alternating { .. } => {
                bench.phasors = Some(bench.circuit.phasors().ok_or_else(stuck)?);
                bench.settle(0.0);
            }
        }
        Ok(bench)
    }
}

impl Bench {
    /// Seconds of the circuit to show in one second of watching.
    pub fn playback(&self) -> f64 {
        match self.scene.analysis {
            Analysis::Steady => 1.0,
            Analysis::Transient { playback, .. } | Analysis::Alternating { playback } => playback,
        }
    }

    /// True once a circuit followed in time has run for as long as it was asked to.
    pub fn finished(&self) -> bool {
        matches!(self.scene.analysis, Analysis::Transient { duration, .. } if self.state.time >= duration - 1e-12)
    }

    /// The alternating state at `time`, from the amplitudes.
    fn settle(&mut self, time: f64) {
        let (Some(phasors), Some(frequency)) = (&self.phasors, self.circuit.frequency()) else {
            return;
        };
        let turn = num_complex::Complex64::from_polar(1.0, TAU * frequency * time);
        self.state = State {
            time,
            potentials: Vec::new(),
            readings: phasors
                .iter()
                .map(|p| Reading {
                    voltage: (p.voltage * turn).im,
                    current: (p.current * turn).im,
                })
                .collect(),
        };
    }

    /// Moves time on by `seconds`. A steady circuit only has its clock moved, which a front end uses to keep
    /// the charges drifting round.
    pub fn advance(&mut self, seconds: f64) -> Result<(), SceneError> {
        match self.scene.analysis {
            Analysis::Steady => self.state.time += seconds,
            Analysis::Alternating { .. } => self.settle(self.state.time + seconds),
            Analysis::Transient { dt, duration, .. } => {
                let until = (self.state.time + seconds).min(duration);
                while self.state.time < until - 0.5 * dt {
                    self.state = self.circuit.step(&self.state, dt).ok_or_else(|| {
                        SceneError(
                            "the circuit stopped having an answer part way through".to_string(),
                        )
                    })?;
                    self.history.push(self.state.clone());
                }
            }
        }
        Ok(())
    }

    fn index(&self, name: &str) -> Result<usize, SceneError> {
        self.circuit.find(name).ok_or_else(|| {
            SceneError(format!(
                "an answer measures `{name}`, which is not in the circuit"
            ))
        })
    }

    /// One quantity of one component at every step so far, for a circuit followed in time.
    pub fn series(&self, element: usize, quantity: Quantity) -> Vec<f64> {
        self.history
            .iter()
            .map(|s| pick(&s.readings[element], quantity))
            .collect()
    }

    /// Each answer's name, textbook value, and what the solved circuit gives for it where it can.
    pub fn answers(&self) -> Result<Vec<(String, f64, Option<f64>)>, SceneError> {
        self.scene
            .answers
            .iter()
            .map(|answer| {
                let measured = match &answer.measure {
                    Some(measure) => self.measure(measure)?.map(|v| v * answer.scale),
                    None => None,
                };
                Ok((answer.name.clone(), answer.value, measured))
            })
            .collect()
    }

    fn measure(&self, measure: &MeasureSpec) -> Result<Option<f64>, SceneError> {
        let now = |name: &str| Ok::<_, SceneError>(self.state.readings[self.index(name)?]);
        let phasor = |name: &str| {
            let index = self.index(name)?;
            Ok::<_, SceneError>(self.phasors.as_ref().map(|p| p[index]))
        };
        Ok(match measure {
            MeasureSpec::Current { element } => Some(now(element)?.current),
            MeasureSpec::Voltage { element } => Some(now(element)?.voltage),
            MeasureSpec::Power { element } => Some(now(element)?.power()),
            MeasureSpec::Charge { element } => {
                let index = self.index(element)?;
                match self.circuit.elements[index].kind {
                    Kind::Capacitor { capacitance, .. } => {
                        Some(capacitance * self.state.readings[index].voltage)
                    }
                    _ => None,
                }
            }
            MeasureSpec::Energy { element } => {
                let index = self.index(element)?;
                let reading = self.state.readings[index];
                match self.circuit.elements[index].kind {
                    Kind::Capacitor { capacitance, .. } => {
                        Some(0.5 * capacitance * reading.voltage * reading.voltage)
                    }
                    Kind::Inductor { inductance, .. } => {
                        Some(0.5 * inductance * reading.current * reading.current)
                    }
                    _ => None,
                }
            }
            MeasureSpec::TimeToReach {
                element,
                quantity,
                value,
            } => {
                let index = self.index(element)?;
                let series = self.series(index, *quantity);
                (1..series.len()).find_map(|k| {
                    let (before, after) = (series[k - 1] - value, series[k] - value);
                    (before * after <= 0.0 && before != after).then(|| {
                        let (t0, t1) = (self.history[k - 1].time, self.history[k].time);
                        t0 + (t1 - t0) * before / (before - after)
                    })
                })
            }
            MeasureSpec::Peak { element, quantity } => {
                let index = self.index(element)?;
                let series = self.series(index, *quantity);
                (series.len() > 1).then(|| series.iter().fold(0.0f64, |most, v| most.max(v.abs())))
            }
            MeasureSpec::Period { element } => {
                let index = self.index(element)?;
                let series = self.series(index, Quantity::Current);
                let rising: Vec<f64> = (1..series.len())
                    .filter(|&k| series[k - 1] < 0.0 && series[k] >= 0.0)
                    .map(|k| {
                        let (t0, t1) = (self.history[k - 1].time, self.history[k].time);
                        t0 + (t1 - t0) * series[k - 1] / (series[k - 1] - series[k])
                    })
                    .collect();
                (rising.len() >= 2)
                    .then(|| (rising[rising.len() - 1] - rising[0]) / (rising.len() - 1) as f64)
            }
            MeasureSpec::Amplitude { element, quantity } => {
                phasor(element)?.map(|p| match quantity {
                    Quantity::Voltage => p.voltage.norm(),
                    Quantity::Current => p.current.norm(),
                })
            }
            MeasureSpec::CurrentLag { current, voltage } => {
                match (phasor(current)?, phasor(voltage)?) {
                    (Some(i), Some(v)) if i.current.norm() > 0.0 => {
                        Some((v.voltage / i.current).arg().to_degrees())
                    }
                    _ => None,
                }
            }
            MeasureSpec::Impedance { voltage, current } => {
                match (phasor(voltage)?, phasor(current)?) {
                    (Some(v), Some(i)) if i.current.norm() > 0.0 => {
                        Some(v.voltage.norm() / i.current.norm())
                    }
                    _ => None,
                }
            }
            MeasureSpec::MeanPower { element } => phasor(element)?.map(|p| p.mean_power()),
        })
    }
}

fn pick(reading: &Reading, quantity: Quantity) -> f64 {
    match quantity {
        Quantity::Voltage => reading.voltage,
        Quantity::Current => reading.current,
    }
}
