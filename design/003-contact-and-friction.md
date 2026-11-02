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

Contacts are created when the gap is below an activation distance of `1e-4` m, so that a block placed on a surface
is in contact from the first step, and so that both corners of a resting box stay in the contact set while it settles.
With a 1 µm margin the box rocked on alternate corners forever: after each position correction one corner sat a hair
above the floor, dropped out of the set, and the predictor's single-corner reaction tipped the box onto the other.
A contact with a positive gap carries no force, so the wider margin makes nothing hover.

## Time stepping

Moreau–Jean style: detect contacts at `x_new` after the Verlet position update and the RATTLE projection of the
bilateral constraints (002), solve all impulses at the velocity level in one Gauss–Seidel sweep together with the
bilateral and rope rows, then push penetrating contacts apart without touching velocities (split impulse).

Why time stepping and not event-driven: event-driven integration (stop exactly at each impact) is precise but fails
on resting contact and on the frequent transitions of friction. Time stepping handles rest, slide, stick and impact
uniformly. Event times are reported at step granularity in Phase 1 and by interpolation in Phase 2.

### The predictor kick

Verlet drifts positions with the half-kicked velocity before any contact impulse exists for the step. Left alone,
a block at rest on an incline creeps downhill by `½ g sin θ dt²` every step, and sliding distances are first order.
So each persistent contact's whole-step reaction from the previous step is applied as half an impulse in the opening
half-kick, exactly where Verlet applies the explicit forces. At rest the kick then balances gravity to round-off and
nothing drifts; while sliding, the deceleration enters the drift and distances become second order. The velocity
solve afterwards still determines the whole step's reaction exactly, so the predictor never changes what a contact
can do, only when within the step it is felt. Impacts are not persistent and never predicted; a contact that ends
leaves one spurious half-step of reaction, an `O(dt)` error at events only, where time stepping is first order anyway.

Contacts are not put through RATTLE's position projection because that would fold the penetration correction into
the pre-impact velocity and corrupt restitution. The split-impulse correction leaves velocities alone.

### Reactions at `t = 0`

Reactions exist only after a solve, so the engine solves one hypothetical step at construction and discards the
motion. This gives the first recorded sample real tensions and normal forces, seeds the predictor so a body placed
on a surface never creeps, and starts the energy ledger's trapezoid from the true initial power.

## Normal impulse and restitution

For each contact row, with relative normal velocity `v_n` before the solve:

```
0 ≤ λ_n  ⊥  (J_n v_new + e · v_n⁻) ≥ 0
```

Newton's restitution with coefficient `e` on the pre-impact velocity. `e = 1` conserves kinetic energy exactly at the
velocity level, so an elastic collision reproduces the textbook outgoing velocities to solver tolerance. Restitution is
applied only when the approach speed exceeds `4 |g| dt + 1e-6`, several times the speed a resting body acquires in
one step, so resting contact never micro-bounces. The pair's coefficient is the smaller of the two bodies' values;
friction coefficients are the larger of the two, so a scene that specifies friction on the surface alone behaves as
the textbook intends.

## Friction

Per contact row, inside the Gauss–Seidel sweep, after the normal impulse is known:

1. Compute the tangential impulse `λ_stick` that would bring relative tangential velocity to zero.
2. If `|λ_stick| ≤ μ_s λ_n`: static friction holds. Apply `λ_stick`. The body does not slide.
3. Otherwise: kinetic friction. Apply `λ_t = -μ_k λ_n · sign(v_t)`.

Static friction shared between several contact points of one body is statically indeterminate, and the solver's
warm start would otherwise carry an arbitrary equal-and-opposite pair forever (a resting box showing ±0.6 N of
friction at its two corners). Each sweep therefore starts friction rows from half the predicted impulse: a leftover
pair decays to zero within a few steps, while a genuinely loaded contact is restored exactly inside the sweep because
the stick target is computed from the actual relative velocity.

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

Friction is reported as a force like any other and enters the same trapezoidal power ledger as drag, which is exact
for a block decelerating uniformly and contributes nothing for a block held at rest. Impacts are different: their
energy change is `-λ (J v⁻ + J v⁺) / 2`, exact for an impulse, and is added at the impact; impact samples are flagged
so the power ledger skips them. `kinetic + potential + dissipated - external_work` then stays constant to the
integrator's accuracy. This turns "where did the energy go" into a number on screen.

## Validation

| Scenario | Checked | Tolerance (measured) |
| --- | --- | --- |
| block on incline, `tan θ < μ_s` | stays at rest, friction = `m g sin θ`, `N = m g cos θ` | 1e-10 (position exactly 0) |
| block on incline, `tan θ > μ_s` | `a = g (sin θ − μ_k cos θ)`, one slip event | 1e-10 |
| block pushed below and above `μ_s N` | at rest with friction = push; then `a = (F − μ_k N)/m` | 1e-10 |
| sliding block stops | distance `u²/2μg`, friction work `½ m u²`, ledger drift | 1e-8 (4e-9, 5e-10, 3e-9) |
| elastic 1D collision, `e = 1` | outgoing velocities, no dissipation, begin and end events | 1e-10 |
| inelastic collision, `e = 0` | common velocity, KE lost equals ledger | 1e-10 |
| oblique elastic collision of equal discs | momentum, energy, 90° scattering | 1e-10 |
| rolling disc on incline, `μ_s` sufficient | `a = g sin θ / (1 + I/mR²)`, `v = ω R`, no slip event | 1e-8 |
| rolling disc, `μ_s` too small | `a = g (sin θ − μ_k cos θ)`, rim lags, one slip event | 1e-8 |
| bouncing ball, `e = 0.8` | successive apex ratios `e²`, first landing time | 5e-5 (measured 2.1e-5), 1e-6 s |
| box on the floor | rests flat, two-point manifold, `N = m g` | 1e-10 |
| ladder on a smooth wall | stays for `μ ≥ 1.3 μ_min`, slides for `0.7 μ_min` | event |

## Impact timing

A contact is found after the drift, so an impact is detected up to one step late with overlap `depth`. The solver
estimates the delay `δ = depth / |v_n|`, corrects the approach speed for the acceleration gained during `δ`, aims the
rebound velocity at what it would be after decelerating for `δ`, places the body where it would be having rebounded
for `δ`, and reports the `ContactBegin` event at `t − δ`. Bounce apex ratios then match `e²` to 3e-5 (from 4e-4) and a
dropped ball's landing time is reported to 1e-6 s.

## Known limits
- A contact that ends receives one spurious half-step of predicted reaction, also `O(dt)` and only at the event.
- Sliding friction uses the velocity at the end of the step for the stick test, so a block comes to rest at a step
  boundary; the position error this causes is below round-off in the tests above.

## Rejected alternatives

- Penalty contact (spring-damper): tunable, stiff, and reports fake normal forces.
- Single friction coefficient: simpler, but the syllabus distinguishes `μ_s` and `μ_k` and so must the free-body diagram.
- Continuous collision detection: not needed at the speeds and sizes in the syllabus with `dt ≤ 1e-3`.
- RATTLE for contacts: exact for resting contact but wrong for impacts; the predictor kick gives resting and sliding
  contact the same second-order behaviour without touching the impact path.
