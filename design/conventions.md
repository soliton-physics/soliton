# Conventions

## Code

- Names say what a thing is. `normal_impulse`, not `lambda_n`; `PinJoint`, not `Constraint2`.
  Physics symbols are fine inside a short formula-shaped function body where the document names the formula.
- One concept per module. A module that needs a paragraph to describe does two things.
- Comments explain physics or a numerical choice. They never restate the code.
  A reference to the design document section is a good comment.
- No `unsafe` in the pillars. Performance comes from algorithms and layout first.
- Public API takes and returns plain data: `f64`, `Vec2`, slices, `Vec`. Handles are small `Copy` ids (`BodyId`).
- Errors are values. Constructing an impossible scene returns an error naming the problem; it never panics.

## Precision

`f64` only. The browser runs `f64` WebAssembly at full speed for these problem sizes, and reproducibility across
Python and the playground matters more than halving memory. If a future need for `f32` or dual numbers appears, it is
a separate design document.

## Determinism

Same scene, same `dt`, same step count gives the same bits on every platform. Practically: no `mul_add`, no
platform-dependent math intrinsics in hot paths, fixed iteration order over bodies and constraints, no hash-map
iteration in physics code.

## Testing and validation

Three kinds of tests, and every scenario has at least the first two:

1. **Closed form.** Compare against the textbook answer: range of a projectile, acceleration of an Atwood machine,
   period of a pendulum. Tolerance is stated in the test and justified by the integrator order.
2. **Invariants.** Energy where it should be conserved, momentum where it should be, with the expected drift bound.
   Where energy is dissipated (friction, inelastic collision) the ledger must balance: work done by friction equals the
   energy lost.
3. **Convergence.** For non-trivial dynamics, halve `dt` and confirm the error falls by the integrator's order.

Tests live next to the pillar in `tests/` and run in CI on every push. A scenario without tests is not in the catalogue.

## Design documents

Short, concrete, and written to be argued with. State the alternatives you rejected and why.
When code lands, update the status and keep the document accurate; it is the reference, not the history.
