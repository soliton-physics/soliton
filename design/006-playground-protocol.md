# 006: What the WebAssembly layer exposes to the playground

Status: Implemented

## Principle

The playground renders; the engine computes. Nothing physical is computed in TypeScript, including the free-body
diagram and the graphs. The WASM surface is the same `Scene` and `Recorder` ideas as the Python package, shaped for a
frame loop.

## Surface

```ts
const engine = await loadEngine();
const catalogue = engine.scenarios();                 // [{ id, title, syllabus, params: [{name, unit, min, max, default, description}] }]
const world = engine.scenario("atwood", { m1: 2, m2: 3 });   // or engine.scene(json)

world.step(n);                                        // advance n fixed steps
const frame = world.snapshot();                       // typed arrays: positions, angles, velocities, shapes, force breakdowns, rope paths
world.exact();                                        // { acceleration: 1.96, tension: 23.52 }
world.events();                                       // events since last call
world.series("block.v.x");                            // Float64Array of a recorded observable, for the graphs panel
```

`snapshot()` returns a handful of `Float64Array`s over a shared layout, not per-body objects, so a frame costs no
allocation beyond the arrays themselves.

As implemented, `MechanicsWorld` exposes: `mechanics_catalogue()`; `new(id, params, record_every)`; `step(n)`,
`time()`, `dt()`; static `bodies()` (label, kind, mass, shape) and `view()`; per frame `state()` as
`[x, y, θ, vx, vy, ω, ax, ay]` per body, `forces(body)` as the free-body diagram, `sketches()`; `exact()`,
`energy()`, `events()` (drained since the previous call); and the recorded `series_time()`, `series(body, quantity)`,
`series_force(body, label)` and `series_energy(quantity)` for the graphs.

`step(n)` returns `true` and stops early on the step the problem ends (design 004), so the playground can pause
exactly there, and once more if everything later comes to rest. `finished()` is `{ time, reason }` or `null`;
`horizon()` is the scene's time window when it has one, which fixes the graphs' time axis up front. Each entry of
`forces(body)` carries `impulsive`, true for an impact impulse spread over one step, which the playground shows in
N·s rather than as a force.

## What the playground draws

- bodies with shapes, velocity and acceleration vectors (toggle)
- the free-body diagram for a selected body, from the engine's force breakdown, with labels and magnitudes
- rope paths and pulleys
- graphs: any recorded observable against time, including x–t, v–t, a–t for the kinematics chapters
- the exact-answer panel: textbook value beside the measured value, with the difference
- the energy bar: kinetic, potential, dissipated, and their sum
- the event log

## Two modes

- Scenario mode: pick from the catalogue, move sliders, the world rebuilds from the scenario function.
- Scene mode: paste or load a scene JSON. A visual scene editor is a later phase.
