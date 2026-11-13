# 011: Charges and fields, for class 12

Status: Implemented (first slice)

## Scope

A charge on a body, three forces that act on it, a way to ask what the field is at a point, and eleven scenarios
from the first chapters of class 12. This is the part of electromagnetism that is mechanics: a charge is a body
with one more number, and the laws that move it are forces like any other.

What is not here is the rest of the subject, and it is a long list: see the end.

## Charge

`Body::charge(q)`, `"charge"` in a scene, in coulombs. The charge sits at the centre of the body. A body with no
charge feels none of the forces below and makes no field.

## Forces

| Force | Law | Label | In the energy ledger |
| --- | --- | --- | --- |
| `Electrostatics` | `F = k q₁q₂/r²` between every pair of charged bodies | `electric` | potential, `k q₁q₂/r` |
| `ElectricField`, filling the plane and steady | `F = qE` | `electric` | potential, `−q E·r` |
| `ElectricField`, in a region or alternating | `F = qE` | `electric` | external work |
| `MagneticField` | `F = q v × B`, `B` along the axis out of the page | `magnetic` | does no work |

**A charged static body is a fixed charge.** It is felt and not moved, the same way the ground is. There is no
separate list of sources as gravitation has (009); a fixed charge can then be drawn, picked and edited like any
other body. Two fixed charges hold an energy between them that nothing can draw on, so it is left out of the
potential energy.

**A field between plates has no potential.** A uniform field that stops dead at the edge of a rectangle is not
conservative: carry a charge up through it and back down outside and you have gained `qEd` for nothing. Real plates
have a fringe that takes it back. The engine has no fringe, so the work such a field does is booked as external,
done by whatever keeps the plates charged, and the ledger still closes: in "speeding up through a potential
difference" the external work is `qV` and all of it is kinetic energy. The same holds for an alternating field,
which is how a cyclotron's supply feeds the charge.

**The magnetic field is perpendicular to the plane.** That is the one arrangement in which a charge that starts in
the plane stays in it. A field in the plane would bend the path out of it: a helix, which needs the third
dimension, as the banked road did in 010.

**The magnetic force depends on velocity**, so it takes the fixed-point closing kick that drag does (001). For a
pure rotation that kick is the trapezoidal rule, which turns the velocity without changing its length: the speed
of a charge in a magnetic field is constant to round-off, not to second order. The radius of the circle is right
to second order in `ω dt`, about one part in 100 000 at four thousand steps a turn.

**A region's edge is first order.** A charge crosses the edge of a region part-way through a step and feels the
field for the whole step or none of it. Scenarios with plates run at `dt = 2.5e-4`, where the deflection is right
to three parts in a thousand.

## Asking about the field

- `World::electric_field(point, time)`: of every charge and every uniform field there, N/C.
- `World::electric_potential(point)`: volts, zero at infinity for charges and at the origin for a uniform field.
  Fields with no potential add nothing.
- `World::magnetic_field(point)`: tesla, positive out of the page.
- `World::fields(time)`: the uniform fields, for drawing.
- `World::field_lines(per_charge, centre, reach, time)`: lines traced along the field by the midpoint rule (the
  rule for where lines start and end was revised in 012). They
  start on positive charges, in number proportional to the charge, and end on a negative charge or leave the
  picture; a negative charge starts lines of its own only for the charge the positive ones cannot account for.

Field lines are a drawing, not a result: how many there are is a choice, and only their direction means anything
exactly. Their density is right near a charge and approximate elsewhere, because in a plane picture of a
three-dimensional field it cannot be right everywhere.

## Scenarios

Charges are those of the laboratory, microcoulombs on beads of a few grams, so forces are millinewtons and the
motion takes seconds. An electron in the same fields follows the same laws a billion times faster; the numbers
here are chosen so the motion can be watched and the forces read.

| Chapter | Scenario | Exact |
| --- | --- | --- |
| Electric charges and fields | force between two charges | `F = kq₁q₂/r²`; speed at `2r` is `√(kq₁q₂/mr)` |
| | field of two charges | `E` and `V` by superposition at a test charge |
| | charged bob in a field | rests at `tan θ = qE/mg`; released from below swings to `2θ` |
| | a charge fired between plates | `y = qEL²/2mv²`; on a screen `D` beyond, `(qEL/mv²)(L/2 + D)` |
| | balancing a charged drop | `E = mg/q`; otherwise `a = qE/m − g` |
| | a dipole in a uniform field | `τ = pE sin θ`; small swings `T = 2π√(I/pE)`; no net force |
| Electrostatic potential and capacitance | closest approach | `½mv² + kqQ/r₀ = kqQ/r_min` |
| | speeding up through a potential difference | `v = √(2qV/m)` whatever the gap |
| Moving charges and magnetism | a charge in a magnetic field | `r = mv/qB`, `T = 2πm/qB` |
| | velocity selector | straight through at `v = E/B` |
| | cyclotron | one turn takes `2πm/qB` at any speed; at most `v = qBR/m` |

Each is tested against its answer in `tests/electromagnetism.rs`, with the conservation law that should hold
beside it.

## Known limits

- **A charge is a point at the centre of its body.** A charged rod or plate is not modelled; a dipole is two
  charged beads on a rod.
- **Charges do not make magnetic fields, and a changing field induces nothing.** Two moving charges feel
  Coulomb's law and no more. That is right to one part in `(v/c)²`, which for a bead is nothing.
- **The cyclotron's magnet is a square, not a pair of dees**, and the charge leaves when its orbit outgrows the
  square, a little before `qBR/m`: the last crossing may or may not happen. The answer says "at most".
- **The period of a dipole is the small-swing formula**; released at 30° it is 2 % longer, as a pendulum's is.

## Not in this slice

Done since, in [012](012-magnetism-and-matter.md): Gauss's law, fixed wires, sheets and shells of charge, bar
magnets and magnetic materials.

Still to come: capacitors and dielectrics; current, resistance and circuits; the field of a current (Biot–Savart,
Ampère), the force between wires; induction, inductance and alternating current. Circuits want a small solver of
their own; induction belongs with the fields pillar.
