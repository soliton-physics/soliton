# 014: Thermal physics

Status: implemented. NCERT class 11, chapters 11 and 12 (thermodynamics, kinetic theory), and the base for the
statistical physics that follows. A new crate, `soliton-thermal`.

## The idea

Thermodynamics is usually simulated by solving its own equations: put in `PV = nRT` and plot it. That shows
nothing a formula does not. Here the gas is molecules, and the thermodynamic quantities are measured from them:

| Quantity | How it is found |
|---|---|
| Temperature | two thirds of the mean kinetic energy of a molecule, over `k` |
| Pressure | momentum handed to the walls, per unit time and area |
| Work | energy gained by molecules bouncing off the moving piston |
| Heat | energy gained by molecules leaving a wall held at a temperature |
| Internal energy | the sum of the kinetic energies |

The first law is then bookkeeping that closes to round-off, and the gas laws, Maxwell's distribution, the
adiabatic exponent and the Carnot efficiency are results.

## Units

Those of molecular dynamics: nm, ps, amu, K. Energy is amu·nm²/ps² (1.66 × 10⁻²¹ J, close to `kT` at room
temperature), `k = 0.0083145`, speeds in nm/ps are km/s. A box 10 nm across with a few hundred atoms is a real gas
at several atmospheres, at its real size and speed.

## The gas

`Gas` holds hard spheres in a rectangular box in three dimensions. One step of `dt`:

1. The piston, the high-x wall, moves if it is being driven.
2. Every molecule flies straight for `dt`.
3. A molecule past a wall and moving into it is turned back. An insulating wall reverses the normal velocity; the
   piston reverses it in the piston's own frame, `v → 2u − v`, and the change of energy is work. A wall at
   temperature `T` gives the molecule a new velocity: Gaussian along the wall, and normal to it drawn from the
   flux distribution `v e^{−mv²/2kT}`, since fast molecules leave a wall more often than slow ones. The change
   of energy is heat.
4. Pairs that overlap and are approaching exchange momentum along their line of centres. Neighbours are found
   with a cell list.

Collisions are detected after the fact, at the end of the step, not at the instant of contact. Energy and
momentum are still conserved exactly in each one; what is approximate is the moment and place, by up to
`v·dt`, about a tenth of an atom's diameter at the default step. An event-driven scheme would be exact there
and much more complicated with a moving piston.

**Walls act on centres.** The volume open to a molecule's centre is then the volume of the box, and the only
departure from `PV = NkT` is the one that is physically there: the molecules exclude one another, raising the
pressure by `2πd³N/3V`, a per cent or two.

**A partition** keeps each molecule to its side until a stage removes it. **Removing a wall suddenly** is done
by moving the piston out in no time: no molecule touches it, so no work is done.

Three dimensions, though the picture is flat, because the textbook numbers are three-dimensional: `3kT/2`,
`γ = 5/3`, `v_rms = √(3kT/m)`. The front end draws depth as size and strength of colour.

## Experiments

A `ThermalScene` is data: the box, groups of molecules (how many, mass, radius, temperature, where), and a list
of stages. A stage sets what the walls do (insulated, all at a temperature, or the two ends at different
temperatures), optionally sends the piston somewhere at a speed, and lasts for a time, or until the piston
arrives, or for ever if it is the last. `Experiment` runs the stages and keeps for each a `StageRecord`: work,
heat, blows on the walls, and running sums of temperature, speeds and left-right share for each kind of
molecule. A sample of the whole gas is recorded every picosecond for graphs.

Answers carry a `MeasureSpec`, the stage it refers to, and a tolerance. Averages are over a stage, so a scene
that wants to measure a gas in equilibrium gives it a stage to settle in first.

## Fluctuations

With `N` molecules the temperature of a gas in contact with a reservoir fluctuates by `√(2/3N)`: 5 % for 250.
This is physics and is shown, but it decides what can be measured:

- **Averages are steady.** Pressure and speeds averaged over a stage converge.
- **Work is steady**, being an integral over a stroke.
- **Heat is not.** `Q = ΔU − W`, and `ΔU` between two instants carries the flicker at both, some 15 to 30 % of
  the heat of an isothermal stroke.
- **So the Carnot efficiency is taken from the two isothermal works,** `1 − W_cold/W_hot`. For an ideal gas at
  fixed temperature the work is the heat, and the two insulated strokes cancel. Taking `W_net/Q_h` from the raw
  ledger gives the right answer only on average, with a spread of 20 % per cycle at this size.

A piston moving at speed `u` meets molecules harder than a slow one and does more work than the reversible
formula, by a fraction of about `1.6 u/√(kT/m)`. At the default 4 m/s that is 2 to 3 %.

## Catalogue

`ideal-gas`, `maxwell-distribution`, `gas-mixture`, `brownian-motion` (kinetic theory); `isothermal-process`,
`adiabatic-process`, `free-expansion`, `mixing`, `carnot-cycle` (thermodynamics).

## Known limits

- Monatomic only: no rotation or vibration, so `γ = 5/3` always.
- No attraction between molecules: no liquid, no critical point.
- Collisions resolved at the end of a step, as above. Three molecules meeting within one step are handled as
  successive pairs.
- A molecule faster than a cell per step could pass another without meeting it; at the default step that would
  take a speed of 6 km/s.
- The piston has no mass and feels no force; its motion is prescribed.

## Next

A potential between molecules (Lennard-Jones) for real gases and phase change; heat conduction with the ends at
different temperatures, for which the stage type already exists; mean free path and diffusion as measured
quantities; then the Ising model and Monte Carlo in a crate of their own.
