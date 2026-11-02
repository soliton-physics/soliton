# 002: Bilateral constraints, pin joints, ropes and pulleys

Status: Draft

## Scope

Equality constraints between bodies and fixed points, solved exactly at the velocity level with a position projection
that removes drift. Ropes are unilateral (they can go slack) and are handled here because they share the machinery.

## Constraints

| Constraint | Equation | Examples |
| --- | --- | --- |
| `PinJoint` | body point = fixed point, or body point = other body point | pendulum pivot, seesaw fulcrum, flywheel axle |
| `Rod` | distance between two points fixed | rigid pendulum, coupled pendula, linkage |
| `Slot` | body point stays on a line | bead on a wire, block confined to a track |
| `FixedAngle` | θ fixed or θ₁ − θ₂ fixed | lock rotation of a body |
| `Rope` | total path length ≤ L, tension ≥ 0 | string, string over pulleys, vertical-circle string |

Each constraint provides its position function `C(x)`, its Jacobian row(s) `J`, and a label for the reaction it
reports (`tension` for ropes and rods, `reaction` for pins).

## Velocity-level solve

After the free velocity `v_free` (001), find impulses `λ` so that

```
J v_new = 0,     v_new = v_free + M⁻¹ Jᵀ λ
```

The system `(J M⁻¹ Jᵀ) λ = -J v_free` is small and symmetric positive semi-definite. With contact (003) it becomes a
mixed complementarity problem. One solver handles both: projected Gauss–Seidel over constraint rows, iterated to a
tolerance of `1e-12` on the residual or a cap of 200 sweeps, with warm starting from the previous step.
For the bilateral rows the projection is the identity, for rope rows `λ ≥ 0`, for contact rows the friction cone.

Why Gauss–Seidel and not a direct solve: the direct solve is also fine for these sizes, but the complementarity
conditions of ropes and contact are not a linear system. One iterative solver for everything keeps the code small,
and at these sizes convergence to `1e-12` costs microseconds. A direct solve for pure-bilateral worlds is a
later optimisation if profiling asks for it.

## Position projection: RATTLE

Velocity constraints alone let positions drift by `O(dt²)` per step. The scheme is RATTLE, which is Verlet with
two projections:

1. After the drift, SHAKE: Gauss–Seidel sweeps move bodies along `M⁻¹ Jᵀ` evaluated at the **start** of the step
   until `|C(x)| < 1e-13`, and the half-step velocities receive the same correction divided by `dt`.
2. After the final kick, the velocity projection above, along `Jᵀ` evaluated at the **new** positions.

Both details matter. Using the current gradient instead of the start-of-step gradient in step 1 looks harmless and
costs a first-order energy error: a pendulum at 2 rad amplitude lost 10 % of its energy over ten periods at
`dt = 1e-4`. With the start-of-step gradient the drift is `1.4e-7` and second order. Skipping the velocity correction in
step 1 (projecting positions only) leaves an `O(dt²)` energy noise several times larger than Verlet's own.

The position sweeps must be Gauss–Seidel: each row's error is re-evaluated after the rows before it have moved the
bodies. Evaluating every error once per sweep is Jacobi iteration, which is not guaranteed to converge on a symmetric
positive-definite system and in fact diverges on a seesaw with two masses pinned to the plank. Re-evaluating costs
one rebuild of the rows per row per sweep, negligible at these sizes.

Bodies joined by a pin or a rod do not collide with each other; otherwise a mass pinned onto a plank fights its own
joint through contact.

No Baumgarte spring term, which would add an artificial frequency a student could see.

## Reaction forces

Each row's position impulse (from SHAKE) plus its velocity impulse, divided by `dt`, is the constraint force for the
step: RATTLE's two multipliers each cover half a step. It is reported in the force breakdown with the constraint's
label and the sign convention `C = L − |d|` for rods and ropes so that tension is positive. For the Atwood machine
this gives the textbook `T = 2 m₁ m₂ g / (m₁ + m₂)` to round-off, and the normal force on a block held by a slot is
exactly `m g`. The position tolerance has to be tight because a position residual `ε` reports as a force error
`ε / dt²`.

## Ropes and pulleys

A rope is a path through nodes:

```
Rope { nodes: [Attach(body_a, point), Fixed(p), Pulley(body_p, radius), Attach(body_b, point)], length }
```

- Path length is the sum of straight segments between consecutive nodes. A `Fixed` node is a frictionless peg.
- A `Pulley` node is a massless pulley: the rope bends around it with the tangent
  geometry of a circle of the given radius, so the segment directions are correct for a pulley of finite size.
- A `Wheel` node is a rigid body the rope wraps without slipping. The rope's tangent points are treated as material
  points of the rim, the rope is split into one row per segment between wheels, and each segment's row gets entries
  for the wheel at its tangent point. The wheel's rotation is then coupled through ordinary point Jacobians, with no
  special no-slip row, and the tensions on the two sides differ by exactly the torque that spins the wheel. Rope
  wound on the rim between the tangent points is assumed to move with the rim, so the wrap angle should not change
  much during a run (true for pinned wheels with fixed-direction segments).
- The rope row is unilateral: `C = L - path_length ≥ 0`, `λ ≥ 0`. When `C > 0` the rope is slack and applies nothing.
  The transition emits `Event::RopeSlack` or `Event::RopeTaut`. A string in a vertical circle goes slack exactly when
  the required tension would be negative, which is the condition students compute.

## Validation

| Scenario | Checked | Tolerance |
| --- | --- | --- |
| Atwood machine | `a = (m₂ − m₁) g / (m₁ + m₂)`, `T` | 1e-10 (measured: round-off) |
| block on table pulled by hanging mass over a pulley | `a`, `T`, `N = m₁ g` | 1e-10 (measured: round-off) |
| simple pendulum (rod) small amplitude | period `2π√(L/g)` | 1e-5 (measured 4e-7); energy drift 1e-10 |
| simple pendulum, 2 rad amplitude | period via the complete elliptic integral | 1e-6 (measured 1.7e-8), order 2.0 |
| string in a vertical circle | slack event only when launched below `√(5 g r)` | event present or absent |
| seesaw with unequal masses | equilibrium when `m₁ d₁ = m₂ d₂`, otherwise angular acceleration | 1e-6 (measured 1e-9) |
| Atwood with a disc pulley | `a = (m₂ − m₁) g / (m₁ + m₂ + M/2)`, `T₁ = m₁(g + a)`, `T₂ = m₂(g − a)`, `v = ω R` | 1e-8 |
| flywheel with a hanging mass | `a = m g / (m + M/2)` | 1e-8 |

## Rejected alternatives

- Springs as constraints (penalty method): simple, but a stiff spring either drifts or forces a tiny `dt`, and the
  reported "tension" is a spring force, not a constraint reaction.
- Reduced coordinates: exact but not composable; see 001.
- Baumgarte stabilisation: adds parameters and a visible artificial oscillation; projection has neither.
