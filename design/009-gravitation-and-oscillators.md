# 009: Gravitation, and oscillators that lose and gain energy

Status: Implemented

## Scope

Newton's law of gravitation as a force, the three gravitation scenarios of the class 11 syllabus (a circular
orbit, an elliptical orbit, escape velocity), and the damped and driven oscillators. The oscillators need nothing
new in the engine: linear drag and a sinusoidal driving force exist since 001. They are here because they arrive
together and share the question of scenes whose natural time is not a few seconds. Coupled oscillators are three
springs and two blocks, again with nothing new underneath.

## Gravitation

```
F = G m₁ m₂ / r²   along the line joining the two,      U = −G m₁ m₂ / r
```

`Gravitation::new(G)` attracts every pair of dynamic bodies. `.source(point, mass)` adds a fixed mass at a point:
it attracts and is not moved. That is the planet a satellite goes round, where the planet's own motion is a part in
10²¹ and a static body has no mass to attract with.

- It reports as `gravity`, like uniform gravity, and is conservative: its potential is in the ledger and an orbit's
  `conserved()` stays constant to the integrator's accuracy.
- Velocity Verlet is symplectic, so an orbit does not spiral in or out however long it runs. The error is in phase
  and shape, second order in `ω dt`: at a few thousand steps a turn a circular orbit's speed is constant to 1e-4
  and its energy to 1e-9.
- Coincident points attract with nothing rather than with infinity. There is no softening length: two bodies that
  reach each other meet through contact (003) like anything else.
- Uniform gravity is a world setting and stays one. A scene in space sets `world.gravity` to zero and adds
  `gravitation`.

In a scene:

```json
{ "gravitation": { "constant": 6.674e-11, "fixed": [ { "at": [0, 0], "mass": 5.972e24 } ] } }
```

Rejected: a "central force" special case with the gravitational parameter `μ = G M` as its one number. It is all
the three scenarios need, but then two stars circling each other would be a second mechanism. Rejected too: giving
static bodies a gravitational mass. A static body's mass is infinite by definition (001), and the planet's disc and
the planet's pull are different facts about it.

## Scenes that take hours

An orbit takes ninety minutes; nobody watches one in real time. Two additions, both hints to front ends:

- `playback` on a scene: seconds of the simulation to show per second of watching. The orbit scenarios set it so
  the problem plays in about fifteen seconds. Absent, it is one.
- The gravitation scenarios choose their time step from the length of the problem, about six thousand steps over
  it, rounded down to 1, 2 or 5 times a power of ten so that it can be read.

The hand (008) follows the time step: its spring's frequency is `min(30 rad/s, 0.5 / dt)`, so in a scene stepped in
seconds it is slower instead of unstable, and its pull is capped as an acceleration, the same 1350 m/s² as before.

## Scenarios

| Scenario | Parameters | Exact |
| --- | --- | --- |
| satellite in a circular orbit | altitude | `v = √(GM/r)`, `T = 2π√(r³/GM)`, `a = GM/r²` |
| elliptical orbit | altitude and speed at the nearest point | `a = −GM/2ε`, `e = r_p v_p²/GM − 1`, farthest point, period, `r v` equal at both ends |
| escape velocity | launch speed, straight up | `v_esc = √(2GM/R)`; below it, maximum height `R / (v_esc²/u² − 1)`; above it, speed far away `√(u² − v_esc²)` |
| damped oscillator | `m, k, b`, starting displacement | whether it oscillates, `T = 2π/√(k/m − γ²)`, half-life of the swing `ln 2 / γ`, `γ = b/2m` |
| driven oscillator and resonance | `m, k, b, F₀, ω` | steady amplitude `F₀/√((k − mω²)² + (bω)²)`, phase lag, where and how large the peak is |
| coupled oscillators | `m, k`, middle spring `k′`, the two starting displacements | `ω₁ = √(k/m)` together, `ω₂ = √((k + 2k′)/m)` against each other, crossing-over time `π/(ω₂ − ω₁)` |

The orbit scenarios run a little over two turns, so the period can be measured between two passes of the same
point. A launch is followed for at most six hours: just below the escape speed it takes days to come back, and
near the ground the time step must stay at a second or two.

Earth: `M = 5.972 × 10²⁴ kg`, `R = 6371 km`, `G = 6.674 × 10⁻¹¹`, which give the textbook's 11.2 km/s.

## Validation

| Check | Tolerance |
| --- | --- |
| circular orbit, 400 km and 20 000 km: speed constant, closes after one period, `a = GM/r²`, energy | 1e-4, one step, 1e-5, 1e-9 |
| elliptical orbit: farthest point, slowest speed | 1e-5 |
| elliptical orbit: areal velocity `r × v / 2` at every sample (second law) | 1e-10 |
| elliptical orbit: back at the nearest point after `T` (third law) | two steps |
| vertical launch at 6 km/s: maximum height; ends on touching the Earth | 1e-5 |
| vertical launch at 12 km/s: speed at 10 Earth radii from energy conservation | 1e-5 |
| two equal free masses circle their midpoint: separation, momentum, energy | 1e-7, 1e-12, 1e-9 |
| damped oscillator against the closed form, every 0.25 s for 6 s; ledger | 1e-5 m, 1e-5 |
| driven oscillator at ω = 2, 4, 7 rad/s: steady amplitude over the last two cycles | 2e-3 |
| coupled oscillators started in each mode against one cosine; started with one block against the sum of two | 2e-5 m, 5e-5 m |

The areal velocity is conserved to round-off, not to the integrator's order: Verlet conserves angular momentum
exactly for a central force. Kepler's second law is the one a student can check on screen to as many digits as
are shown.

## Not here yet

- A resonance curve (amplitude against driving frequency) as a graph: it needs a sweep of runs, which is a
  recorder feature of its own.

In the playground's sandbox the force is a setting beside the downward pull: a strength `G` chosen by the user,
because the real one moves nothing between bodies of a few kilograms. A static body does not attract; a heavy body
that should stay put is pinned.
