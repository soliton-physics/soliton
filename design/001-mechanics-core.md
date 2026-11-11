# 001: Mechanics core

Status: Implemented

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
| `Torque` | one body | constant couple; flywheels and pivoted rods |
| `Gravitation` | pairs of bodies | Newton's law; Phase 2 |
| `UniformElectric`, `UniformMagnetic`, `Coulomb` | charged bodies | Lorentz and Coulomb; on hold until requested |

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

Velocity-dependent forces (drag, magnetic) make the final kick implicit: `v_new` appears on both sides. Evaluating
them at `v_half` instead would drop the scheme to first order. The engine solves the final kick by fixed-point
iteration, `v ← v_half + (dt/2) a(x_new, v, t + dt)`, which converges in a few iterations for the damping strengths in
the syllabus and keeps second order. Worlds without velocity-dependent forces skip the iteration. The convergence test
for the damped oscillator guards this.

Forces are evaluated at step boundaries, and one evaluation serves both the closing half-kick of a step and the
opening half-kick of the next. A force that switches on or off exactly at a boundary would then be applied for half a
step too long or too short. Forces therefore receive which half-step they are being evaluated for, and a time window
`[from, until)` tests membership of that half-step rather than of the instant. A push of 3 N for 2 s gives exactly
the textbook speed. Worlds with windowed forces re-evaluate loads at the start of each step; others reuse the previous
end-of-step loads. Time is `steps × dt`, never accumulated, so a window edge at `t = 2` is hit exactly.

Phase 1 ships Verlet only. Alternatives planned by name: `Rk4` for smooth, dissipative, unconstrained systems where
symplecticity does not matter and fourth order helps (resonance curves); `Yoshida4` for conservative orbital problems
when a long integration needs fourth order and symplecticity together. Neither will support constraints or contact.

## What the engine reports

Per body, at every sample:

- kinematics: `x, θ, v, ω, a, α`
- force breakdown: a list of `(label, F, application_point)` including constraint and contact reactions from 002
  and 003, labelled `tension`, `normal`, `friction`. This is the free-body diagram.
- energies: kinetic (translational and rotational), gravitational potential relative to a chosen datum, elastic

Per world:

- total linear momentum, total angular momentum about the origin
- an energy ledger: kinetic, potential, cumulative energy dissipated by drag and friction and lost in inelastic
  impacts, and cumulative work done by applied forces, torques and driving forces. The conserved quantity is
  `kinetic + potential + dissipated − external_work`, and its drift is the accuracy measure. Work is accumulated by the
  trapezoidal rule over each step so the ledger is second order like the integrator.
- events (see 002 and 003)

## Recorder

`Recorder::every(n_steps)` attaches to a `World` and appends samples to columnar buffers: one `Vec<f64>` per
quantity per body. Force breakdowns are stored as `(label_id, fx, fy, px, py)` rows in a flat table with a per-sample
offset, so a notebook can ask for "the normal force on body 2 over time" and get a contiguous array.

## Public API (Rust)

```rust
let mut world = World::new(Vec2::new(0.0, -9.8));
let ground = world.add(Body::fixed(HalfPlane::new(Vec2::Y, 0.0)).friction(0.4, 0.3));
let block  = world.add(Body::particle(1.0).at(0.0, 0.0).velocity(3.0, 4.0));
world.add_force(Applied::new(block, Vec2::new(2.0, 0.0)).until(1.5));

let mut recorder = Recorder::every(10);
let mut sim = Simulation::new(world, 1e-3);
sim.run_until(3.0, &mut recorder);

recorder.body(block).position;                   // Vec<Vec2>
recorder.force(block, ForceLabel::Normal);        // Vec<Vec2>, one per sample
sim.energy().conserved();                         // kinetic + potential + dissipated − external work
sim.events();                                     // &[Event]
```

Bodies are values built with a fluent API and handed to `World::add`, which returns the `BodyId`. Materials
(friction coefficients, restitution) live on the body; the defaults are frictionless and perfectly elastic, which is
what a textbook means by "smooth" and "elastic" when it says nothing.

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
