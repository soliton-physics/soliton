use crate::linear::{solve, Scalar};
use num_complex::Complex64;
use std::f64::consts::TAU;

/// What a component is. Every one joins two junctions, `a` and `b`; its voltage is the potential at `a` less
/// that at `b`, and its current is the current through it from `a` to `b`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    /// A connecting wire, with no resistance.
    Wire,
    Resistor {
        resistance: f64,
    },
    /// A cell with its positive terminal at `a`: an EMF in series with an internal resistance.
    Battery {
        emf: f64,
        internal: f64,
    },
    Capacitor {
        capacitance: f64,
        /// The voltage across it at the start.
        initial_voltage: f64,
    },
    Inductor {
        inductance: f64,
        /// The current through it at the start.
        initial_current: f64,
    },
    /// An alternating source: `amplitude · sin(2π · frequency · t + phase)` volts.
    Source {
        amplitude: f64,
        frequency: f64,
        phase: f64,
    },
    /// Open or closed. `flips_at` is a time at which it changes over, if it does.
    Switch {
        closed: bool,
        flips_at: Option<f64>,
    },
    /// Reads the current through it and has no resistance.
    Ammeter,
    /// Reads the voltage across it and passes no current.
    Voltmeter,
    /// A sensitive meter with a resistance of its own, for finding where no current flows.
    Galvanometer {
        resistance: f64,
    },
    /// A p–n junction, conducting from `a` to `b`. `saturation` is the tiny current that leaks the wrong way,
    /// A. With `breakdown`, it also conducts backwards once the reverse voltage reaches that many volts: a
    /// Zener diode.
    Diode {
        saturation: f64,
        breakdown: Option<f64>,
    },
}

/// The thermal voltage `kT/e` at room temperature, V: the scale on which a junction's current grows.
pub const THERMAL_VOLTAGE: f64 = 0.025_85;
/// The saturation current of a small silicon diode and of a germanium one, A.
pub const SILICON: f64 = 1e-12;
pub const GERMANIUM: f64 = 1e-6;
/// The current at which a Zener diode's breakdown voltage is quoted, A.
const ZENER_TEST: f64 = 5e-3;
/// A conductance across every junction, far too small to matter, that keeps the equations settled when a
/// diode is passing nothing, S.
const LEAK: f64 = 1e-12;

/// The current through a junction with `voltage` across it, and how fast that current grows with the voltage:
/// Shockley's equation, `I = Iₛ(e^{V/V_T} − 1)`, and for a Zener the same law mirrored about its breakdown.
pub fn junction(saturation: f64, breakdown: Option<f64>, voltage: f64) -> (f64, f64) {
    // Beyond this the exponential is carried on as a straight line, which a solver's first wild guesses need.
    let grow = |x: f64| {
        if x > 40.0 {
            let top = 40.0f64.exp();
            (top * (1.0 + x - 40.0), top)
        } else {
            (x.exp(), x.exp())
        }
    };
    let (forward, slope) = grow(voltage / THERMAL_VOLTAGE);
    let mut current = saturation * (forward - 1.0) + LEAK * voltage;
    let mut conductance = saturation * slope / THERMAL_VOLTAGE + LEAK;
    if let Some(volts) = breakdown {
        // Placed so that the reverse current is the test current at exactly the quoted voltage.
        let knee = volts - THERMAL_VOLTAGE * (ZENER_TEST / saturation).ln();
        let (reverse, slope) = grow(-(voltage + knee) / THERMAL_VOLTAGE);
        current -= saturation * reverse;
        conductance += saturation * slope / THERMAL_VOLTAGE;
    }
    (current, conductance)
}

#[derive(Clone, Debug, PartialEq)]
pub struct Element {
    pub name: String,
    pub a: usize,
    pub b: usize,
    pub kind: Kind,
}

/// Junctions numbered from zero, and the components between them. Junction 0 is where potential is measured
/// from.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Circuit {
    pub nodes: usize,
    pub elements: Vec<Element>,
}

/// What is going on in one component at one moment.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Reading {
    /// Volts, `a` above `b`.
    pub voltage: f64,
    /// Amperes, from `a` to `b` through the component.
    pub current: f64,
}

impl Reading {
    /// Watts taken from the circuit by the component; negative for one that is supplying.
    pub fn power(&self) -> f64 {
        self.voltage * self.current
    }
}

/// The steady alternating voltage and current of one component, each as a complex amplitude: the quantity
/// itself is the imaginary part of the amplitude times `e^{iωt}`, so a phase of zero is a sine.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Phasor {
    pub voltage: Complex64,
    pub current: Complex64,
}

impl Phasor {
    /// Watts taken on average over a cycle.
    pub fn mean_power(&self) -> f64 {
        0.5 * (self.voltage * self.current.conj()).re
    }
}

/// The circuit at an instant: time, the potential at each junction, and a reading for each component.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct State {
    pub time: f64,
    pub potentials: Vec<f64>,
    pub readings: Vec<Reading>,
}

/// How a component enters the equations. Every one is either a conductance with a current source beside it, or
/// a branch whose current is an unknown of its own and whose voltage is fixed by an equation.
enum Stamp<T> {
    /// Passes nothing and fixes nothing: an open switch, a voltmeter.
    Open,
    /// `i = conductance · v + source`.
    Conductance { conductance: T, source: T },
    /// `v − resistance · i = emf`.
    Branch { resistance: T, emf: T },
}

/// The potential at each junction, and the voltage and current of each component.
type Solved<T> = (Vec<T>, Vec<(T, T)>);

/// The most improvements of a diode's guessed voltage before giving up.
const NEWTON_STEPS: usize = 400;

/// Resistance of a capacitor's plates and leads at the starting instant, Ω.
const PLATES: f64 = 1e-9;

/// How time enters the equations for a capacitor or an inductor.
#[derive(Clone, Copy)]
enum Moment<'a> {
    /// Currents that have stopped changing: a capacitor passes nothing, an inductor is a wire.
    Steady,
    /// The very start: a capacitor holds its voltage and an inductor its current, whatever else happens.
    Start,
    /// One step of `dt` on from `before`, by the trapezoidal rule.
    Step { dt: f64, before: &'a State },
}

impl Circuit {
    pub fn new(nodes: usize) -> Self {
        Self {
            nodes,
            elements: Vec::new(),
        }
    }

    pub fn add(&mut self, name: &str, a: usize, b: usize, kind: Kind) -> usize {
        self.elements.push(Element {
            name: name.to_string(),
            a,
            b,
            kind,
        });
        self.elements.len() - 1
    }

    pub fn find(&self, name: &str) -> Option<usize> {
        self.elements.iter().position(|e| e.name == name)
    }

    /// Whether a switch is closed at `time`.
    fn closed(closed: bool, flips_at: Option<f64>, time: f64) -> bool {
        closed != flips_at.is_some_and(|at| time >= at)
    }

    fn stamp(&self, index: usize, time: f64, moment: Moment, guess: &[f64]) -> Stamp<f64> {
        let wire = Stamp::Branch {
            resistance: 0.0,
            emf: 0.0,
        };
        match self.elements[index].kind {
            Kind::Wire | Kind::Ammeter => wire,
            Kind::Voltmeter => Stamp::Open,
            Kind::Resistor { resistance } | Kind::Galvanometer { resistance } => {
                Stamp::Conductance {
                    conductance: 1.0 / resistance,
                    source: 0.0,
                }
            }
            // Its curve replaced by the tangent to it at the voltage guessed: Newton's method, a step at a time.
            Kind::Diode {
                saturation,
                breakdown,
            } => {
                let (current, conductance) = junction(saturation, breakdown, guess[index]);
                Stamp::Conductance {
                    conductance,
                    source: current - conductance * guess[index],
                }
            }
            // v = emf + i·r, with i reckoned from the positive terminal to the negative through the cell.
            Kind::Battery { emf, internal } => Stamp::Branch {
                resistance: internal,
                emf,
            },
            Kind::Source {
                amplitude,
                frequency,
                phase,
            } => Stamp::Branch {
                resistance: 0.0,
                emf: amplitude * (TAU * frequency * time + phase).sin(),
            },
            Kind::Switch { closed, flips_at } => {
                if Self::closed(closed, flips_at, time) {
                    wire
                } else {
                    Stamp::Open
                }
            }
            Kind::Capacitor {
                capacitance,
                initial_voltage,
            } => match moment {
                Moment::Steady => Stamp::Open,
                // Its plates have a trace of resistance, far too little to matter, so that two capacitors
                // wired side by side do not state the same thing twice and leave the currents unsettled.
                Moment::Start => Stamp::Branch {
                    resistance: PLATES,
                    emf: initial_voltage,
                },
                // i = C dv/dt, with both sides averaged over the step.
                Moment::Step { dt, before } => {
                    let conductance = 2.0 * capacitance / dt;
                    let was = before.readings[index];
                    // A capacitor put straight across a cell is charged in no time by a surge without limit.
                    // That surge is over the moment it has happened, and is not carried into the average.
                    let surge = was.current.abs() * PLATES > 1e-6 * (1.0 + was.voltage.abs());
                    let current = if surge { 0.0 } else { was.current };
                    Stamp::Conductance {
                        conductance,
                        source: -conductance * was.voltage - current,
                    }
                }
            },
            Kind::Inductor {
                inductance,
                initial_current,
            } => match moment {
                Moment::Steady => wire,
                Moment::Start => Stamp::Conductance {
                    conductance: 0.0,
                    source: initial_current,
                },
                // v = L di/dt, with both sides averaged over the step.
                Moment::Step { dt, before } => {
                    let resistance = 2.0 * inductance / dt;
                    let was = before.readings[index];
                    Stamp::Branch {
                        resistance,
                        emf: -was.voltage - resistance * was.current,
                    }
                }
            },
        }
    }

    /// Writes Kirchhoff's laws for the stamps given and solves them: the potential at each junction, and the
    /// voltage and current of each component.
    fn kirchhoff<T: Scalar + From<f64>>(&self, stamps: &[Stamp<T>]) -> Option<Solved<T>> {
        // Unknowns: the potential of every junction but the first, then the current of every branch.
        let free = self.nodes.saturating_sub(1);
        let mut branch = vec![usize::MAX; stamps.len()];
        let mut size = free;
        for (i, stamp) in stamps.iter().enumerate() {
            if matches!(stamp, Stamp::Branch { .. }) {
                branch[i] = size;
                size += 1;
            }
        }
        let mut a = vec![vec![T::ZERO; size]; size];
        let mut rhs = vec![T::ZERO; size];
        let row = |node: usize| node.checked_sub(1);
        // A part of the circuit cut off from junction 0, by an open switch say, has potentials fixed only up
        // to a constant. One of its junctions is tied to junction 0; no current can flow through the tie, so
        // nothing else changes.
        let mut group: Vec<usize> = (0..self.nodes).collect();
        fn root(group: &mut [usize], mut node: usize) -> usize {
            while group[node] != node {
                group[node] = group[group[node]];
                node = group[node];
            }
            node
        }
        for (i, stamp) in stamps.iter().enumerate() {
            if !matches!(stamp, Stamp::Open) {
                let (p, q) = (
                    root(&mut group, self.elements[i].a),
                    root(&mut group, self.elements[i].b),
                );
                // The lower number becomes the root, so that junction 0 is the root of its own part.
                group[p.max(q)] = p.min(q);
            }
        }
        for node in 1..self.nodes {
            if root(&mut group, node) == node {
                a[node - 1][node - 1] += T::from(1.0);
            }
        }
        for (i, stamp) in stamps.iter().enumerate() {
            let (p, q) = (row(self.elements[i].a), row(self.elements[i].b));
            match *stamp {
                Stamp::Open => {}
                Stamp::Conductance {
                    conductance,
                    source,
                } => {
                    // The current leaves `a` and arrives at `b`.
                    if let Some(p) = p {
                        a[p][p] += conductance;
                        rhs[p] -= source;
                        if let Some(q) = q {
                            a[p][q] -= conductance;
                        }
                    }
                    if let Some(q) = q {
                        a[q][q] += conductance;
                        rhs[q] += source;
                        if let Some(p) = p {
                            a[q][p] -= conductance;
                        }
                    }
                }
                Stamp::Branch { resistance, emf } => {
                    let k = branch[i];
                    if let Some(p) = p {
                        a[p][k] += T::from(1.0);
                        a[k][p] += T::from(1.0);
                    }
                    if let Some(q) = q {
                        a[q][k] -= T::from(1.0);
                        a[k][q] -= T::from(1.0);
                    }
                    a[k][k] -= resistance;
                    rhs[k] = emf;
                }
            }
        }
        let x = solve(a, rhs)?;
        let potential = |node: usize| row(node).map_or(T::ZERO, |r| x[r]);
        let potentials: Vec<T> = (0..self.nodes).map(potential).collect();
        let readings = stamps
            .iter()
            .enumerate()
            .map(|(i, stamp)| {
                let voltage = potential(self.elements[i].a) - potential(self.elements[i].b);
                let current = match *stamp {
                    Stamp::Open => T::ZERO,
                    Stamp::Conductance {
                        conductance,
                        source,
                    } => conductance * voltage + source,
                    Stamp::Branch { .. } => x[branch[i]],
                };
                (voltage, current)
            })
            .collect();
        Some((potentials, readings))
    }

    fn at(&self, time: f64, moment: Moment) -> Option<State> {
        let count = self.elements.len();
        let diode = |i: usize| matches!(self.elements[i].kind, Kind::Diode { .. });
        // A diode's voltage is guessed, the circuit solved with its tangent, and the guess improved, until it
        // stops changing. A step in time starts from where the diode was.
        let mut guess = match moment {
            Moment::Step { before, .. } => before.readings.iter().map(|r| r.voltage).collect(),
            _ => vec![0.0; count],
        };
        for _ in 0..NEWTON_STEPS {
            let stamps: Vec<Stamp<f64>> = (0..count)
                .map(|i| self.stamp(i, time, moment, &guess))
                .collect();
            let (potentials, readings) = self.kirchhoff(&stamps)?;
            let mut moved: f64 = 0.0;
            for i in (0..count).filter(|i| diode(*i)) {
                let (was, wanted) = (guess[i], readings[i].0);
                // The exponential is so steep that a full step overshoots wildly; a climb is taken as far as
                // the tangent's logarithm allows, as circuit simulators do.
                let rise = (wanted - was).abs();
                let limited = THERMAL_VOLTAGE * (1.0 + rise / THERMAL_VOLTAGE).ln();
                guess[i] = was + limited.copysign(wanted - was);
                moved = moved.max(rise);
            }
            if moved < 1e-10 {
                return Some(State {
                    time,
                    potentials,
                    readings: readings
                        .into_iter()
                        .map(|(voltage, current)| Reading { voltage, current })
                        .collect(),
                });
            }
        }
        None
    }

    /// The steady state: what the meters read once every capacitor has finished charging and every inductor's
    /// current has stopped changing. `None` if Kirchhoff's laws do not settle it.
    pub fn steady(&self) -> Option<State> {
        self.at(0.0, Moment::Steady)
    }

    /// The instant the circuit is switched on, with each capacitor at its starting voltage and each inductor
    /// carrying its starting current.
    pub fn start(&self) -> Option<State> {
        self.at(0.0, Moment::Start)
    }

    /// The state `dt` seconds after `before`.
    pub fn step(&self, before: &State, dt: f64) -> Option<State> {
        let time = before.time + dt;
        // A switch changing over is a break in everything; the step that crosses it is taken from the
        // conditions just before, with nothing averaged across the break.
        let thrown = self.elements.iter().any(|e| {
            matches!(e.kind, Kind::Switch { flips_at: Some(at), .. } if before.time < at && time >= at)
        });
        if thrown {
            return self.after_break(before, time);
        }
        self.at(time, Moment::Step { dt, before })
    }

    /// The state just after a switch is thrown: capacitors keep their voltages and inductors their currents,
    /// and everything else rearranges itself round them at once.
    fn after_break(&self, before: &State, time: f64) -> Option<State> {
        let mut held = self.clone();
        for (element, was) in held.elements.iter_mut().zip(&before.readings) {
            match &mut element.kind {
                Kind::Capacitor {
                    initial_voltage, ..
                } => *initial_voltage = was.voltage,
                Kind::Inductor {
                    initial_current, ..
                } => *initial_current = was.current,
                _ => {}
            }
        }
        held.at(time, Moment::Start)
    }

    /// The frequency of the alternating sources, if there are any.
    pub fn frequency(&self) -> Option<f64> {
        self.elements.iter().find_map(|e| match e.kind {
            Kind::Source { frequency, .. } => Some(frequency),
            _ => None,
        })
    }

    /// The steady alternating state at the frequency of the sources: for each component, the complex
    /// amplitudes of its voltage and current. Cells count only as their internal resistance, and switches are
    /// as they start.
    pub fn phasors(&self) -> Option<Vec<Phasor>> {
        let omega = TAU * self.frequency()?;
        let real = |v: f64| Complex64::new(v, 0.0);
        let wire = || Stamp::Branch {
            resistance: real(0.0),
            emf: real(0.0),
        };
        let stamps: Vec<Stamp<Complex64>> = self
            .elements
            .iter()
            .map(|element| match element.kind {
                Kind::Wire | Kind::Ammeter => wire(),
                // A diode has no single impedance; the alternating solution is for circuits without one.
                Kind::Voltmeter | Kind::Diode { .. } => Stamp::Open,
                Kind::Resistor { resistance } | Kind::Galvanometer { resistance } => {
                    Stamp::Conductance {
                        conductance: real(1.0 / resistance),
                        source: real(0.0),
                    }
                }
                Kind::Battery { internal, .. } => Stamp::Branch {
                    resistance: real(internal),
                    emf: real(0.0),
                },
                Kind::Source {
                    amplitude, phase, ..
                } => Stamp::Branch {
                    resistance: real(0.0),
                    emf: Complex64::from_polar(amplitude, phase),
                },
                Kind::Switch { closed, .. } => {
                    if closed {
                        wire()
                    } else {
                        Stamp::Open
                    }
                }
                // I = iωC V.
                Kind::Capacitor { capacitance, .. } => Stamp::Conductance {
                    conductance: Complex64::new(0.0, omega * capacitance),
                    source: real(0.0),
                },
                // V = iωL I.
                Kind::Inductor { inductance, .. } => Stamp::Branch {
                    resistance: Complex64::new(0.0, omega * inductance),
                    emf: real(0.0),
                },
            })
            .collect();
        let (_, readings) = self.kirchhoff(&stamps)?;
        Some(
            readings
                .into_iter()
                .map(|(voltage, current)| Phasor { voltage, current })
                .collect(),
        )
    }
}
