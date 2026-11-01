# 004: Scenes, parameters, and the NCERT scenario catalogue

Status: Draft

## Scenes are data

A scene is a JSON document that fully describes a world. The playground loads it, the Python package loads it, and
the Rust builder API produces it. Teachers share scenes as files.

```json
{
  "schema": 1,
  "title": "Block on an incline",
  "syllabus": { "class": 11, "chapter": "Laws of Motion", "topic": "friction" },
  "world": { "gravity": [0, -9.8], "dt": 0.001 },
  "bodies": [
    { "id": "incline", "kind": "static", "shape": { "polygon": [[0,0],[4,0],[0,2.3]] }, "friction": [0.4, 0.3] },
    { "id": "block", "kind": "particle", "mass": 2, "position": [0.5, 2.1], "shape": { "circle": 0.05 } }
  ],
  "forces": [ { "applied": { "body": "block", "force": [5, 0], "until": 2 } } ],
  "constraints": [],
  "observables": ["block.a", "block.force.normal", "block.force.friction", "energy.total"]
}
```

The schema is versioned. `soliton-core` validates and upgrades old versions.

## Parameters

A catalogue scenario is a Rust function from parameters to a scene, plus a parameter schema:

```rust
pub fn atwood(p: &Params) -> Scene
// params: m1 (kg, 0.1..20, default 2), m2 (kg, 0.1..20, default 3), g (m/s², default 9.8)
```

The schema (name, unit, range, default, description) is what the playground turns into sliders and what the Python
package documents. Changing a slider rebuilds the scene from the function; there is one source of truth for a scenario.

## Closed-form answers

Each scenario exposes `exact(p: &Params) -> Vec<(name, value)>`: the textbook quantities for those parameters.
The playground shows them beside the measured values. The tests assert they agree.

## Catalogue, class 11

Ordered by the NCERT chapters. Each row is a scene builder with parameters, an exact answer, and a test.

### Motion in a straight line

| Scenario | Parameters | Exact |
| --- | --- | --- |
| uniform motion and uniform acceleration with x–t, v–t, a–t graphs | `u, a, t` | kinematic equations |
| free fall and ball thrown upward | `u, h, g` | apex, time of flight |
| stopping distance | `u, μ_k` | `u² / (2 μ_k g)` |
| relative motion of two bodies | `u₁, u₂, separation` | meeting time |

### Motion in a plane

| Scenario | Parameters | Exact |
| --- | --- | --- |
| projectile from ground | `u, θ` | `R, H, T` |
| projectile from a height | `u, θ, h` | range and landing time |
| projectile on an incline | `u, θ, α` | range along the incline |
| uniform circular motion (bead on a wire) | `r, v` | `a = v²/r` |

### Laws of motion

| Scenario | Parameters | Exact |
| --- | --- | --- |
| block on frictionless incline | `m, θ` | `a = g sin θ`, `N = m g cos θ` |
| block on incline with friction | `m, θ, μ_s, μ_k` | rest or `a = g(sin θ − μ_k cos θ)` |
| block pushed on a rough floor | `m, F, μ_s, μ_k` | slip threshold, `a` |
| two blocks connected by a string, pulled | `m₁, m₂, F` | `a, T` |
| Atwood machine | `m₁, m₂` | `a, T` |
| block on a table and a hanging mass | `m₁, m₂, μ` | `a, T` |
| lift accelerating (apparent weight) | `m, a_lift` | `N = m(g + a)` |
| recoil (gun and bullet) | `M, m, v_bullet` | recoil speed |
| car on a level circular road | `r, μ_s` | `v_max = √(μ_s g r)` |
| banked road | `r, θ, μ_s` | speed range |
| conical pendulum | `L, θ` | `T_period`, tension |

### Work, energy and power

| Scenario | Parameters | Exact |
| --- | --- | --- |
| work–energy on a rough floor | `m, u, μ_k` | distance to stop, work by friction |
| spring compressed by a sliding block | `m, u, k` | max compression |
| ball dropped onto a spring | `m, h, k` | max compression |
| elastic and inelastic collisions in 1D | `m₁, m₂, u₁, u₂, e` | final velocities, KE lost |
| oblique collision of two discs | `m, u, impact parameter` | scattering angles |
| bob in a vertical circle | `L, u_bottom` | slack condition, `√(5 g L)` |
| power of a constant force | `F, m` | `P = F v` |

### System of particles and rotational motion

| Scenario | Parameters | Exact |
| --- | --- | --- |
| centre of mass of a system of particles | masses, positions | centre of mass, its motion under external force |
| seesaw equilibrium and tipping | `m₁, d₁, m₂, d₂` | balance condition, `α` |
| torque on a pivoted rod | `τ, I` | `α = τ / I` |
| rolling disc, ring and sphere down an incline | shape, `θ, μ_s` | `a = g sin θ / (1 + I / m R²)` |
| flywheel driven by a hanging mass (Phase 2, massive pulley) | `M, R, m` | `a`, tension |
| ladder against a wall | `L, θ, μ_floor` | minimum `μ` |

### Gravitation (Phase 2)

| Scenario | Parameters | Exact |
| --- | --- | --- |
| circular orbit | `M, r` | period, speed |
| elliptical orbit | `M, r_p, v_p` | Kepler's laws, energy |
| escape velocity | `M, R, u` | escape or bound |

### Oscillations

| Scenario | Parameters | Exact |
| --- | --- | --- |
| mass on a horizontal spring | `m, k, A` | `T = 2π √(m/k)` |
| vertical spring | `m, k` | equilibrium shift, same `T` |
| simple pendulum, small and large amplitude | `L, θ₀` | `T`, elliptic-integral correction |
| springs in series and parallel | `k₁, k₂, m` | effective `k` |
| damped oscillator | `m, k, b` | envelope `e^{−bt/2m}`, damped frequency |
| driven oscillator and resonance (Phase 2) | `m, k, b, F₀, ω_d` | amplitude vs `ω_d` |

## Catalogue, class 12 (on hold until requested)

| Scenario | Parameters | Exact |
| --- | --- | --- |
| charged particle in a uniform electric field | `q, m, E, u` | parabolic deflection |
| charged particle in a uniform magnetic field | `q, m, B, u` | radius `m v / q B`, cyclotron period |
| velocity selector | `E, B` | undeflected speed `E / B` |
| two charges (Coulomb) | `q₁, q₂, m` | trajectories, energy |

## Validation policy for the catalogue

A scenario is in the catalogue only when its test passes against `exact`. The table in the docs site is generated from
the same source, so what readers see is what CI checked.
