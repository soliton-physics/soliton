# 007: Spectral field solvers, baseline

Status: Implemented (baseline), with follow-ups

## What exists

Periodic pseudo-spectral solvers for semi-linear equations `u_t = L u + N(u)` with ETDRK4 time stepping, implemented
for Korteweg–de Vries and the focusing nonlinear Schrödinger equation, with exact solitons for validation and tracked
invariants. See the docs site for the method and the current error figures.

With the crate split in `architecture.md` this code moves from `soliton-core` into `soliton-fields` unchanged.

## Follow-ups, in order

1. Sine-Gordon and the second-order family (state becomes a pair; Strang splitting or a symplectic stepper)
2. Modified KdV, Boussinesq, Benjamin–Ono, Camassa–Holm
3. Dealiasing (2/3 rule), spectral filters, and a step-size controller with the same fixed-step reproducibility rules
4. Two-dimensional periodic grids: 2D NLS, Gross–Pitaevskii with vortices, Kadomtsev–Petviashvili
5. Non-periodic domains: Chebyshev collocation, sponge layers
6. Diagnostics: spectra, soliton detection, Zakharov–Shabat eigenvalues for soliton content
7. A chain of masses and springs in `mechanics` whose continuum limit is the wave equation: the bridge lesson

Each becomes its own design document when its turn comes.
