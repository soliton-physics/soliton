# 010: The rest of the class 11 catalogue

Status: Implemented

## Scope

Thirteen more scenarios, which with the twenty-two before them cover the mechanics of the class 11 syllabus that
can be shown in a plane. Two small additions to the engine that they needed, and a plain statement of what is left
out and why.

## Scenarios

| Chapter | Scenario | Parameters | Exact |
| --- | --- | --- | --- |
| Motion in a straight line | stopping distance | `u, μ_k` | `s = u²/2μ_k g`, `t = u/μ_k g` |
| Motion in a plane | projectile on an incline | `u`, angle above the slope `β`, slope `α` | `R = 2u² sin β cos(α+β) / g cos²α`, `T = 2u sin β / g cos α`, greatest range `u²/g(1+sin α)` |
| Motion in a plane | uniform circular motion, from above | `r, v, m` | `a = v²/r`, `T = mv²/r`, period `2πr/v` |
| Laws of motion | two blocks joined by a string | `m₁, m₂, F, μ` | `a = F/(m₁+m₂) − μg`, `T = m₁F/(m₁+m₂)` |
| Laws of motion | apparent weight in a lift | `m, a` | `N = m(g + a)` |
| Laws of motion | recoil of a gun | `M, m, v` | `V = mv/M`, energy share `M/(M+m)` |
| Work, energy and power | a block runs into a spring | `m, u, k` | `x = u√(m/k)` |
| Work, energy and power | a ball dropped onto a spring | `m, h, k` | `x = (mg + √((mg)² + 2k·mg·h))/k` |
| Work, energy and power | a glancing collision of equal discs, from above | `u`, offset | right angle; `sin φ = b/2R`, `v₂ = u cos φ`, `v₁ = u sin φ` |
| Rotation | torque on a pivoted rod, from above | `τ, M, L` | `α = τ / (ML²/3)` |
| Oscillations | a mass on a vertical spring | `m, k` | stretch `mg/k`, same period, lowest point `2mg/k` |
| Oscillations | springs in series | `k₁, k₂, m` | `k = k₁k₂/(k₁+k₂)` |
| Oscillations | springs in parallel | `k₁, k₂, m` | `k = k₁ + k₂` |

Three of them are seen from above, with `world.gravity` zero: gravity then points into the picture and does nothing
in it. A stone on a string, discs on a smooth table and a rod on a pivot are all honestly planar that way.

## A spring with a free end

A block that runs into a spring is not attached to it. `Spring::pushing_only()`, `"pushes_only": true` in a scene,
is a spring that pushes when it is squeezed shorter than its natural length and does nothing when its ends are
further apart. It is drawn at its natural length, pointing at what it will meet, until then.

It replaces the obvious model, a light plate on the end of an ordinary spring for the block to hit. That model was
built first and rejected: a plate needs a mass, which costs energy at the impact; and a spring pressing a very
light plate against a heavy block makes the contact between them chatter, hundreds of touch and leave events in one
squeeze. The textbook's spring has no mass and no plate, and now neither has the engine's.

The force switches on part-way through a time step, a kink that Verlet is first order across. The compression is
right to about one part in 100 000 at `dt = 1e-3`.

## What counts as an impact

Restitution, and the treatment of a contact as a blow and not a steady push (003), applied whenever two bodies
approached faster than `4 |g| dt`: several times the speed a resting body picks up from gravity in a step.

Gravity is not the only thing that presses bodies together. A spring pushing a 2 g plate against a 1 kg block
would give the plate alone 15 000 m/s², so every step the plate approached the block at 15 m/s and every step was
taken for an impact. Impacts are not predicted, so the contact dropped to first order, gained 6 % in energy over one
squeeze and booked 80 J as dissipated.

The threshold is now set per contact, from the forces on that pair:

```
approach speed  >  4 · max(|g|, |a_rel|) · dt + 1e-6
```

where `a_rel` is the relative acceleration along the normal that the explicit forces would give the two bodies. For
anything resting under gravity nothing changes; the sixteen scenarios of the first release reproduce the same
numbers to the last digit. A spring pressing a plate on a block is a push again, second order, with the energy
ledger intact. `tests/contact.rs` holds the case.

## Not included, and why

- **Car on a level circular road, banked road, conical pendulum.** Each is motion in a horizontal circle looked at
  from the side, which needs the third dimension. They could be drawn in a frame turning with the car, with a
  centrifugal force added, but class 11 teaches these in the ground frame and does not use that force, and a scene
  that shows it would be teaching something else. Uniform circular motion seen from above is the planar part of
  the same idea, and is in.
- **Relative motion of two bodies, centre of mass of a system, power of a constant force.** Listed in 004 and not
  built; none needs anything new.

## Known limits these scenarios show

- **A glancing collision is right to about a percent.** The right angle, momentum and energy hold to round-off.
  How the speed is shared depends on where the discs touch, and a contact is noticed up to one step late with the
  discs a few millimetres into each other. Working back to the moment of touching (003, impact timing) corrects
  when the impact happened but not which way the line of centres pointed then.
- **The angular acceleration reported for a body turning about a pin is first order.** The motion is second
  order: a rod under a steady torque reaches the right speed to eight figures. The figure reported at an instant
  includes the pin's reaction, and for a turning body that is off by about `ω dt`, a third of a percent in the rod
  scenario. Front ends show the growth of the speed over time.
- **The join of two springs in series has a little mass**, a five-hundredth of the block's, which lengthens the
  period by about a part in a thousand.
- **A projectile on a slope starts 2 mm clear of it**, so that it begins free. That adds a few ten-thousandths of
  a second to its flight.

## Validation

| Check | Tolerance |
| --- | --- |
| stopping distance and the time to stop | 1e-6, 2 ms |
| projectile on three slopes: time of flight, range, greatest range | 5e-4, 8 cm, exact |
| whirled stone: speed, radius, tension, acceleration | 1e-9, 1e-12, 1e-5, 1e-5 |
| two blocks: acceleration and tension, with and without friction | 1e-8 |
| lift at +2, 0, −3 and −9.8 m/s²: the floor's push on the person | 1e-6 N |
| recoil: both speeds, total momentum, work done by the charge | 1e-10, 1e-9, 1e-6 |
| block into a spring: compression, greatest force, return speed, energy | 1e-4 |
| ball onto a spring at three heights and stiffnesses: compression, energy | 1e-4 |
| glancing collision: right angle, momentum, energy; angle and speeds | 1e-9; 0.5°, 1 % |
| rod under a torque: speed and angle after 2 s, work equals kinetic energy | 1e-8, 1e-5, 1e-6 |
| vertical spring: lowest point, period, greatest speed | 1e-5 |
| springs in parallel and in series: period | 1e-5, 2e-3 |
| a spring pressing a light plate on a block: compression, energy, nothing booked as dissipated | 3e-3, 1e-2 |
