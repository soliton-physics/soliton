# 016: Monte Carlo and the Ising model

Status: implemented. The first step beyond the school syllabus into statistical and computational physics, asked
for by a university teacher. A new crate, `soliton-statmech`.

## Scope

Systems followed by chance. Three models, each small:

- **Darts.** Points uniform in the unit square; the fraction inside a quarter circle estimates `π/4`. The whole of
  Monte Carlo integration in one picture, with its `1/√N` error.
- **Walkers.** Many independent walkers on a square grid, each stepping to one of four neighbours at random. The
  root mean square distance after `N` steps is `√N`: diffusion.
- **Ising.** A square lattice of spins `±1` with nearest-neighbour coupling `J`, periodic at the edges, in a
  field `h`, updated by the Metropolis rule.

Units for the lattice: temperature in `J/k`, energy in `J`.

## Metropolis

One sweep is `L²` proposals. Each picks a site uniformly at random and computes the cost of flipping it,
`ΔE = 2 s (Σ neighbours + h)`. The flip is made if `ΔE ≤ 0`, and otherwise with probability `e^{−ΔE/kT}`.
Random-site updates are used and not a fixed order, for the plainness of the rule; a checkerboard order would be
faster and equally correct.

Averages are taken of `|m|` and of `e`, one sample a sweep, after a stated number of sweeps to settle. The
specific heat and susceptibility come from the fluctuations, `N(⟨e²⟩ − ⟨e⟩²)/T²` and `N(⟨m²⟩ − ⟨|m|⟩²)/T`.

## What it is checked against

Onsager's solution of the infinite square lattice at `h = 0`: `kT_c = 2J / ln(1 + √2)`, Yang's magnetisation
`(1 − sinh⁻⁴ 2βJ)^{1/8}`, and the energy per spin
`−J coth 2βJ [1 + (2/π)(2 tanh² 2βJ − 1) K(κ)]` with `κ = 2 sinh 2βJ / cosh² 2βJ`. The elliptic integral is
computed by the arithmetic-geometric mean, and the formula is written with `tanh` and `1/cosh` so that it stays
finite at low temperature.

A finite lattice differs from that in two known ways. `⟨|m|⟩` is not zero above `T_c` but of order `1/L`, and the
transition is rounded over a range of temperature that shrinks as `L` grows. Tolerances allow for both, and the
docs say so.

## A temperature sweep

`Model::Sweep` takes the lattice through a list of temperatures. At each it settles, then measures, then records
a `Point` (temperature, magnetisation, energy, specific heat, susceptibility) and moves on. Cooling from hot, it
starts disordered; heating from cold, ordered. The temperature at which the specific heat is largest is the
measured critical temperature.

Near `T_c` the Metropolis rule decorrelates slowly (critical slowing down), so points there carry more error
than their sweep count suggests. A cluster algorithm would fix it and is not built.

## Random numbers

The generator is the seeded xoshiro256++ of `soliton-thermal`. Every run is reproducible from its seed.

## Catalogue

`monte-carlo-pi`, `random-walk`, `ising-model`, `phase-transition`. Their syllabus class is 0: not a school
class.

## Next

Wolff clusters and finite-size scaling of the critical exponents; percolation; the two-dimensional XY model;
and molecular dynamics with a real potential in `soliton-thermal`.
