# 019: Currents and magnetism

Status: implemented. NCERT class 12: the part of moving charges and magnetism that is about the field of a
current and the force on one, the chapter on electromagnetic induction, and the transformer. A new crate,
`soliton-magnetism`. Charges moving in given fields, and bar magnets, are in mechanics (designs 010 and 012).

## Shape

Thirteen lessons behind the `Lesson` trait of design 017, with the readings, graphs and answers of
`soliton-modern` and a `Picture` enum of its own.

## Two things computed independently of the formulas

- **Fields from the Biot–Savart law.** `biot_savart::field_of_path` sums `(μ₀/4π) I dl × r / r³` over straight
  pieces of the wire, each taken at its midpoint. A long wire is 200 distances either way in 40 000 pieces,
  which leaves out about one part in 10⁵; a turn is 720 pieces. The formulas `μ₀I/2πr`, the field on the axis
  of a coil, the finite solenoid's `½μ₀nI(cos θ₁ + cos θ₂)` and the force between parallel wires are each
  compared with such a sum. The compass needles in the pictures come from the same sums, off the axis too.
- **emf from the change of flux.** Each induction lesson knows the flux through its circuit as a function of
  where things are, and takes the emf as `−ΔΦ/Δt` between one step and the next. The peak emf of a magnet
  through a coil, `NBAω`, `L I/T` and `MωI₀` are compared with that.

## What is stepped in time

- **Galvanometer.** A coil on a spring in a radial field, so the torque `NIAB` does not depend on its angle.
  It swings with a period of 1.2 s at 0.45 of critical damping, by semi-implicit Euler at 1 ms. The period and
  the damping are chosen to be watched; the steady deflection `NIAB/k` does not depend on them.
- **A magnet through a coil.** The magnet is a point dipole on the axis, whose flux through a turn is
  `μ₀mR²/2(R² + z²)^{3/2}`, carried at a steady speed from six radii one side to six the other. The largest
  emf is at half a radius from the plane. The charge sent round while it comes in is `NΦ/R` whatever the
  speed, and the test holds the lesson to that.
- **Rod on rails.** `m dv/dt = F − B²l²v/R` by fourth-order Runge–Kutta over seven time constants, with the
  distance and the heat by Simpson's rule. The pull's work is compared with the heat and the kinetic energy
  all the way along.
- **A loop falling through a band of field.** Braked only while one horizontal edge is in the field and the
  other is not. While braked the speed closes exponentially on `mgR/B²l²`, and that step is taken exactly,
  with the distance and the heat integrated in closed form, so a very strongly braked loop is as steady as a
  free one. The heat is compared with the potential energy lost less the kinetic energy gained.
- **Generator, self-induction, mutual induction.** A prescribed rotation or current, and the emf from the
  change of flux. The self-inductance lesson also adds up the work done against the emf while the current
  rises and compares it with `½LI²`.

Each runs in its own time and is shown in a fixed number of seconds of watching; the clock is in the lesson's
own time.

## Left out

- The solenoid's flux for self and mutual inductance is that of a long solenoid, `μ₀nI` across the whole
  section; the ends are not counted there, as they are in the field lesson.
- The magnet is a point dipole and the coil is flat; a real magnet as long as the coil's radius gives a flatter,
  wider pulse.
- The loop and the rails have no inductance of their own, so their currents follow the emf at once.
- The transformer is ideal: no leakage, no resistance in the windings, no losses in the core.
- No motor with a commutator, no eddy currents in a solid plate, no toroid.
