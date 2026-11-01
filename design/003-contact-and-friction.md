# 003: Contact, Coulomb friction, restitution, time stepping

Status: Draft

## Scope

Non-penetration between shapes, Coulomb friction with distinct static and kinetic coefficients, and impacts with a
coefficient of restitution. This is the part of the syllabus where simulators usually cheat. We do not.

## Contact detection

Shapes: `Circle`, convex `Polygon`, `HalfPlane`. Pairs: circle–circle, circle–polygon, polygon–polygon (separating
axis), anything–half-plane. All pairs are tested every step; with tens of bodies there is no need for a broad phase.

A contact has a point, a unit normal from body A to body B, and a penetration depth (negative when separated).
A polygon resting on a plane produces a two-point manifold, which is what lets a box rest flat and lets a block on a
steep incline tip over instead of sliding, a classic question.

Contacts are created when the gap is below a small activation distance (`1e-6` m by default) so that a block placed on
a surface is in contact from the first step, not after falling into it.

## Time stepping

Moreau–Jean style: detect contacts at `x_new` after the Verlet position update (001), solve all impulses at the
velocity level together with the bilateral rows (002), then project positions. Penetration between steps is at most
`|v| dt` and is removed by the projection, so resting contact is clean.

Why time stepping and not event-driven: event-driven integration (stop exactly at each impact) is precise but fails
on resting contact and on the frequent transitions of friction. Time stepping handles rest, slide, stick and impact
uniformly. Event times are reported at step granularity in Phase 1 and by interpolation in Phase 2.

## Normal impulse and restitution

For each contact row, with relative normal velocity `v_n` before the solve:

```
0 ≤ λ_n  ⊥  (J_n v_new + e · v_n⁻) ≥ 0
```

Newton's restitution with coefficient `e` on the pre-impact velocity. `e = 1` conserves kinetic energy exactly at the
velocity level, so an elastic collision reproduces the textbook outgoing velocities to solver tolerance. Restitution is
applied only when `v_n⁻` exceeds a threshold (`1e-3` m/s) so a resting ball does not micro-bounce.

## Friction

Per contact row, inside the Gauss–Seidel sweep, after the normal impulse is known:

1. Compute the tangential impulse `λ_stick` that would bring relative tangential velocity to zero.
2. If `|λ_stick| ≤ μ_s λ_n`: static friction holds. Apply `λ_stick`. The body does not slide.
3. Otherwise: kinetic friction. Apply `λ_t = -μ_k λ_n · sign(v_t)`.

This is exactly the reasoning a student is taught, encoded per contact. Consequences that fall out without special
cases:

- A block on an incline stays at rest when `tan θ ≤ μ_s`, and slides with `a = g (sin θ − μ_k cos θ)` otherwise.
- A pushed block does not move until the push exceeds `μ_s N`, then accelerates under `μ_k N`.
- A disc released on an incline rolls without slipping when `μ_s ≥ tan θ / (1 + m R² / I)`, with
  `a = g sin θ / (1 + I / m R²)`, and otherwise slips. Rolling is not a special mode; it is friction doing its job.

Transitions between static and kinetic friction emit `Event::FrictionSlip` and `Event::FrictionStick`.
Static friction is reported in the force breakdown as `friction` with its actual magnitude, which is what free-body
diagrams in the textbook show and what students most often get wrong.

## Dissipation ledger

Friction impulses do work. Each step adds `-λ_t · v_t` (always non-negative) to the world's dissipated-energy ledger,
and inelastic impacts add the kinetic energy they remove. `KE + PE + dissipated` must then stay constant to the
integrator's accuracy. This turns "where did the energy go" into a number on screen.

## Validation

| Scenario | Checked | Tolerance |
| --- | --- | --- |
| block on incline, `tan θ < μ_s` | stays at rest, friction = `m g sin θ` | 1e-10 |
| block on incline, `tan θ > μ_s` | `a = g (sin θ − μ_k cos θ)` | 1e-10 |
| block pushed with increasing force | slips when `F = μ_s N` | event at `dt` |
| elastic 1D collision, `e = 1` | outgoing velocities | 1e-10 |
| inelastic collision, `e = 0` | common velocity, KE lost equals ledger | 1e-10 |
| 2D collision of discs | momentum conserved, scattering angles | 1e-10 |
| rolling disc on incline | `a = g sin θ / (1 + I/mR²)`, no slip event | 1e-8 |
| bouncing ball, `e < 1` | successive heights `h_{n+1} = e² h_n` | second-order convergence |
| ladder against a wall | stays until `μ` below the textbook threshold | event |

## Rejected alternatives

- Penalty contact (spring-damper): tunable, stiff, and reports fake normal forces.
- Single friction coefficient: simpler, but the syllabus distinguishes `μ_s` and `μ_k` and so must the free-body diagram.
- Continuous collision detection: not needed at the speeds and sizes in the syllabus with `dt ≤ 1e-3`.
