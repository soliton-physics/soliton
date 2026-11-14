# 015: Circuits

Status: implemented. NCERT class 12: chapter 3 (current electricity), the capacitor half of chapter 2, the
inductor of chapter 6, and chapter 7 (alternating current). A new crate, `soliton-circuits`.

## The idea

A circuit is junctions and the components between them. Two things are true of every one:

1. The currents into a junction add to zero.
2. Each component relates the voltage across it to the current through it.

The crate writes these down for whatever circuit it is given and solves them (modified nodal analysis). Nothing
else is known to it: not the rule for resistors in series, not the balance condition of a bridge, not `RC`, not
the impedance of an LCR circuit. Those are what the solutions show, and the catalogue checks each against its
formula.

## Conventions

A component joins junctions `a` and `b`. Its voltage is the potential at `a` less that at `b`; its current runs
from `a` to `b` through it. A resistor's voltage and current have the same sign. A cell has its positive
terminal at `a`, so one that is supplying has a negative current. Junction 0 is the reference for potential.

## Components

Each is one of two things in the equations:

- a **conductance** with a current source beside it, `i = G v + s`;
- a **branch** whose current is an unknown of its own and whose voltage obeys `v − R i = E`.

| Component | Steady | In time | Alternating |
|---|---|---|---|
| wire, ammeter, closed switch | branch, `R = 0`, `E = 0` | same | same |
| voltmeter, open switch | nothing | same | same |
| resistor, galvanometer | conductance `1/R` | same | same |
| cell | branch, `R = r`, `E = ε` | same | branch, `R = r`, `E = 0` |
| alternating source | branch, `E = A sin(ωt + φ)` | same | branch, `E = A e^{iφ}` |
| capacitor | nothing | see below | conductance `iωC` |
| inductor | a wire | see below | branch, `R = iωL` |

The unknowns are the potential of every junction but the first and the current of every branch. The equations
are solved by elimination with partial pivoting; the systems are a few dozen unknowns. A singular system means
Kirchhoff's laws do not settle the circuit (a loop of ideal cells), and the scene is refused with words to that
effect.

A part of the circuit cut off from junction 0 by open switches has potentials fixed only up to a constant. The
crate finds such parts and ties one junction of each to the reference; no current can flow through the tie.

## In time

A capacitor's `i = C dv/dt` and an inductor's `v = L di/dt` are applied over a step with both sides averaged
(the trapezoidal rule). That turns a capacitor into a conductance `2C/dt` beside a source made of its voltage and
current at the previous step, and an inductor into a branch of resistance `2L/dt` and an EMF likewise. The rule
is second order and, for an LC circuit, keeps the energy to round-off.

The **first instant** is solved separately: each capacitor holds its starting voltage and each inductor carries
its starting current, and everything else arranges itself round them. A capacitor is given 10⁻⁹ Ω there, so
that two wired side by side do not state one fact twice. A **switch thrown part way** is treated the same way:
the step that crosses it starts afresh from the voltages and currents held just before.

## Alternating

The steady alternating state is found once, with complex amplitudes at the frequency of the sources. A quantity
is the imaginary part of its amplitude times `e^{iωt}`, so a phase of zero is a sine. Front ends animate that
state; the switching-on is not shown. A test follows the same LCR circuit step by step until its start has died
away and finds the same amplitude to `1e-4`.

## Scenes

A `CircuitScene` gives each junction a place on the page, so the same data is the diagram. The scenario code
builds it with a small `Board` on which components are put between points, and points at the same place are the
same junction. Wires are components like any other, which costs an unknown each and keeps the drawing and the
circuit one thing. A resistor may be marked as a length of resistance wire, to be drawn as wire: the two parts
of a bridge wire either side of the jockey.

Answers carry a `MeasureSpec`: a voltage, current, power, charge or energy now; the time a quantity first
reaches a value; a peak; a period from zero crossings; and for alternating circuits an amplitude, the lag of a
current behind a voltage, an impedance, and a mean power.

## Catalogue

Nineteen scenarios: `ohms-law`, `resistors-in-series`, `resistors-in-parallel`, `internal-resistance`,
`cells-in-parallel`, `kirchhoff-rules`, `wheatstone-bridge`, `meter-bridge`, `potentiometer`;
`charging-capacitor`, `discharging-capacitor`, `capacitors-in-series`, `capacitors-in-parallel`; `lr-circuit`;
`lc-oscillations`, `ac-resistor`, `ac-inductor`, `ac-capacitor`, `series-lcr`.

## Known limits

- Linear components only: no diode, no lamp whose resistance rises as it heats.
- No mutual inductance, so no transformer.
- Meters are ideal, wires have no resistance, and nothing has stray capacitance.
- One frequency in an alternating circuit.
- The trapezoidal rule can ring for a step or two after an abrupt change in a circuit with an inductor and an
  open switch; no scenario does that.
