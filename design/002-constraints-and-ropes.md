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

## Position projection

Velocity constraints alone let positions drift by `O(dt²)` per step. After the velocity solve, a few Gauss–Seidel
sweeps on the position constraints move bodies along `M⁻¹ Jᵀ` until `|C(x)| < 1e-10`. This is the SHAKE idea
applied after Verlet, so the pair is second order and keeps the constraint manifold exactly. No Baumgarte spring
term, which would add an artificial frequency a student could see.

## Reaction forces

The impulse on each row divided by `dt` is the constraint force for that step. It is reported in the force breakdown
with the constraint's label and sign convention chosen so that tension is positive. For the Atwood machine this gives
the textbook `T = 2 m₁ m₂ g / (m₁ + m₂)` to solver tolerance.

## Ropes and pulleys

A rope is a path through nodes:

```
Rope { nodes: [Attach(body_a, point), Fixed(p), Pulley(body_p, radius), Attach(body_b, point)], length }
```

- Path length is the sum of straight segments between consecutive nodes. A `Fixed` node is a frictionless peg.
- A `Pulley` node with `no_slip: false` (Phase 1) is a massless pulley: the rope bends around it with the tangent
  geometry of a circle of the given radius, so the segment directions are correct for a pulley of finite size.
- A `Pulley` node with `no_slip: true` (Phase 2) adds a row coupling the rope speed to the pulley's rim speed
  `ω R`, so a massive pulley accelerates with the blocks and tension differs on its two sides, as in NCERT's flywheel
  and massive-pulley problems.
- The rope row is unilateral: `C = L - path_length ≥ 0`, `λ ≥ 0`. When `C > 0` the rope is slack and applies nothing.
  The transition emits `Event::RopeSlack` or `Event::RopeTaut`. A string in a vertical circle goes slack exactly when
  the required tension would be negative, which is the condition students compute.

## Validation

| Scenario | Checked | Tolerance |
| --- | --- | --- |
| Atwood machine | `a = (m₂ − m₁) g / (m₁ + m₂)`, `T` | 1e-10 |
| block on table pulled by hanging mass over a pulley | `a`, `T` | 1e-10 |
| simple pendulum (rod) small amplitude | period `2π√(L/g)` | `(ω dt)²/24` plus amplitude correction |
| simple pendulum large amplitude | period via the complete elliptic integral | second-order convergence |
| string in a vertical circle | slack event when `v² < g r` at the top; min bottom speed `√(5 g r)` | event time to `dt` |
| seesaw with unequal masses | equilibrium when `m₁ d₁ = m₂ d₂`, otherwise angular acceleration | 1e-10 |

## Rejected alternatives

- Springs as constraints (penalty method): simple, but a stiff spring either drifts or forces a tiny `dt`, and the
  reported "tension" is a spring force, not a constraint reaction.
- Reduced coordinates: exact but not composable; see 001.
- Baumgarte stabilisation: adds parameters and a visible artificial oscillation; projection has neither.
