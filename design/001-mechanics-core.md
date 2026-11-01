# 001: Mechanics core

Status: Draft

## Scope

Planar rigid-body dynamics with explicit forces. Constraints are 002, contact is 003. This document fixes the state,
the force model, the integrator, and what the engine reports.

## State

A body is

| Field | Symbol | Notes |
| --- | --- | --- |
| position | `x` | centre of mass, metres |
| angle | `θ` | radians, counter-clockwise |
| velocity | `v` | m/s |
| angular velocity | `ω` | rad/s |
| mass | `m` | kg; `Static` bodies have infinite mass and never move |
| moment of inertia | `I` | about the centre of mass; `Particle` bodies have rotation locked |
| shape | | `Circle`, `Polygon` (convex), `HalfPlane`; used by contact and for drawing |

Three kinds of body cover the syllabus: `Particle` (a block treated as a point, the default for most problems),
`Rigid` (a disc, a rod, a box that can rotate), and `Static` (ground, incline, wall, pivot).
Inertia for standard shapes is computed from the shape and mass; the user can override it.

Storage is structure-of-arrays inside `World`: positions, velocities and masses in parallel `Vec`s, indexed by `BodyId`.
Bindings read whole columns.

## Forces

A force is anything that contributes `(F, τ)` to a body given the current state and time. Each carries a label used in
the free-body diagram.

| Force | Acts on | Formula |
| --- | --- | --- |
| `Gravity` | every dynamic body | `m g` |
| `Applied` | one body | constant vector, optional time window, optional application point |
| `Spring` | two anchors | Hooke, `-k (ℓ - ℓ₀)` along the line; anchors are a body point or a fixed point |
| `LinearDrag` | one body | `-b v` |
| `QuadraticDrag` | one body | `-c |v| v` |
| `Driving` | one body | `F₀ cos(ω_d t)` |
| `Gravitation` | pairs of bodies | Newton's law; Phase 2 |
| `UniformElectric`, `UniformMagnetic`, `Coulomb` | charged bodies | Lorentz and Coulomb; Phase 2 |

Forces are stateless. Anything that needs memory (a rope that is slack, a contact that persists) is a constraint.

## Integrator

Default is velocity Verlet, written to accommodate the impulses of 002 and 003:

```
v_half = v + (dt/2) a(x, v, t)
x_new  = x + dt v_half
a_new  = a(x_new, v_half, t + dt)
v_free = v_half + (dt/2) a_new
v_new  = v_free + M⁻¹ Jᵀ λ          impulses from constraints and contacts (002, 003)
```

Why Verlet: it is symplectic and time-reversible, so energy in a pendulum, a spring or an orbit oscillates about the
true value instead of drifting. It is exact for constant acceleration, so a projectile trajectory is reproduced to
round-off at any `dt`. It is second order for everything else.

Velocity-dependent forces (drag, magnetic) make Verlet formally implicit. We evaluate them at `v_half`, which keeps
second order in practice for the damping strengths in the syllabus. The convergence test for damped oscillation
guards this.

Alternatives available by name: `Rk4` for smooth, dissipative, unconstrained systems where symplecticity does not
matter and fourth order helps (resonance curves); `Yoshida4` for conservative orbital problems when a long integration
needs fourth order and symplecticity together. Neither supports constraints or contact in Phase 1.

## What the engine reports

Per body, at every sample:

- kinematics: `x, θ, v, ω, a, α`
- force breakdown: a list of `(label, F, application_point)` including constraint and contact reactions from 002
  and 003, labelled `tension`, `normal`, `friction`. This is the free-body diagram.
- energies: kinetic (translational and rotational), gravitational potential relative to a chosen datum, elastic

Per world:

- total linear momentum, total angular momentum about the origin
- total mechanical energy and a dissipation ledger: cumulative work done by friction and drag, energy lost in inelastic
  impacts. `KE + PE + dissipated` is the quantity that must stay constant, and its drift is the accuracy measure.
- events (see 002 and 003)

## Recorder

`Recorder::every(n_steps)` attaches to a `World` and appends samples to columnar buffers: one `Vec<f64>` per
quantity per body. Force breakdowns are stored as `(label_id, fx, fy, px, py)` rows in a flat table with a per-sample
offset, so a notebook can ask for "the normal force on body 2 over time" and get a contiguous array.

## Public API (Rust)

```rust
let mut world = World::new(Gravity::new(0.0, -9.8));
let ground = world.add_static(HalfPlane::new(Vec2::Y, 0.0)).friction(0.4, 0.3);
let block  = world.add_particle(1.0).at(0.0, 0.0).velocity(3.0, 4.0);
world.add_force(Applied::new(block, Vec2::new(2.0, 0.0)).until(1.5));

let mut recorder = Recorder::every(10);
let mut sim = Simulation::new(world, Verlet, 1e-3);
sim.run_until(3.0, &mut recorder);

recorder.body(block).x();              // &[f64]
recorder.body(block).force("normal");  // &[Vec2]
sim.events();                           // &[Event]
```

## Validation (Phase 1)

| Scenario | Checked quantity | Tolerance | Why |
| --- | --- | --- | --- |
| projectile, no drag | range, apex, landing time | 1e-12 relative, time to `dt` | Verlet exact for constant `a`; time located at step granularity until Phase 2 interpolation |
| mass on spring | period over 100 cycles | `(ω dt)²/24` relative | Verlet period error |
| damped oscillator | amplitude envelope | second-order convergence | guards the `v_half` approximation |
| free rigid body with torque | `ω(t) = τ t / I` | 1e-12 | exact for constant `α` |

## Rejected alternatives

- Semi-implicit Euler (what game engines use): first order; a pendulum's energy drifts visibly in a lesson.
- Generalised coordinates per scenario: exact and simple per case, but every new scenario is new code and teachers
  cannot compose. The engine must be general.
- Adaptive time steps: better accuracy per cost, but every run then has a different step sequence, which complicates
  reproducibility and the playground's fixed frame budget. Users choose `dt`; convergence tests tell them it is enough.
