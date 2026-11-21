# 018: Properties of matter

Status: implemented. NCERT class 11, the chapters on mechanical properties of solids, mechanical properties of
fluids, and thermal properties of matter. A new crate, `soliton-matter`.

## Shape

Sixteen lessons behind the `Lesson` trait of design 017: `advance`, `finished`, `still`, `clock`, and a picture,
readings, graphs and answers. The readings, graphs and answers are the types of `soliton-modern`, which this
crate depends on for them; the `Picture` enum is its own, since the pictures are.

## What is stepped in time

- **Floating.** A cube on a vertical line, under weight, the upthrust on however much of it is under, and a drag
  set a little below critical so that it bobs once or twice and stops. Semi-implicit Euler at 0.5 ms. The
  surface does not rise as the cube goes in: a wide tank.
- **Draining.** `dh/dt = −(a/A)√(2gh)` by the midpoint rule. The level approaches zero as the square of the time
  left, so the last ten-thousandth is finished by that rule and not by stepping, which would never arrive.
- **Falling.** `dv/dt = (1 − σ/ρ)g − v/τ` with `τ = 2r²ρ/9η`, by fourth-order Runge–Kutta with steps of `τ/20`.
  The added mass of the liquid dragged along is left out.
- **Venturi.** Tracers advected at `v₁A₁/A(x)`. One is timed through the throat.
- **Mixing, heating, cooling.** Two heat capacities exchanging heat in proportion to their difference; enthalpy
  accumulated at constant power and mapped to temperature and phase; `dT/dt = −k(T − Tₛ)` by Runge–Kutta.
- **Conduction.** Forty slices, explicit in time, the flux between two slices given by Fourier's law across the
  two half slices in series, so that a join between metals needs no special case. The step is a fifth of the
  stability limit. Both metals are given the same heat capacity per volume, which changes how fast the rod
  settles and nothing about where.

Each of these runs in its own time and is shown in a fixed number of seconds of watching; the clock is in the
lesson's own time.

## What is evaluated

Hooke's law with a sketched curve beyond the elastic limit (forty times the compliance, fracture at 1.6 times
the limit), shear, bulk compression under sea water, pressure at a depth, Pascal's law, capillary rise, thermal
expansion, and Planck's law. For Planck the peak is found by search and the area by summation in the logarithm
of the wavelength, and these are compared with Wien's and Stefan's laws.

## Magnification

Elastic strains are a thousandth or less. Pictures of solids exaggerate by a stated factor (60 for stretch, 500
for shear, 10 for bulk, 200 for expansion); the numbers never do.

## Next

Flow round a body with streamlines, Poiseuille flow, convection cells, and a bimetallic strip.
