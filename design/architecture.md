# Architecture

## Purpose

One engine, three doors. The same Rust code answers a researcher in a notebook, a student in a browser, and a reader
in the docs. There is no simplified version for teaching. What makes the engine educational is not that it is simpler,
but that it explains itself: it reports forces, energies, constraint reactions and events as first-class outputs.

## Two pillars

| Pillar | Models | State | Method |
| --- | --- | --- | --- |
| `mechanics` | rigid bodies in a plane, forces, constraints, contact | finite set of bodies | symplectic time stepping with impulses |
| `fields` | continuous fields on a grid, wave equations | arrays of field values | Fourier pseudo-spectral, exponential integrators |

They share the ideas below but not their internals. A pendulum and a soliton have nothing in common numerically,
and pretending otherwise would make both worse.

## Crates

```
crates/
  soliton-core        shared: scalar and vector types, Invariant and Event records, Recorder, Simulation trait
  soliton-mechanics   the mechanics pillar
  soliton-fields      the fields pillar (today's soliton-core content moves here)
  soliton-wasm        wasm-bindgen surface over both pillars
  soliton-py          PyO3 surface over both pillars; the `soliton` Python package
```

Dependency direction is strictly downward. Pillars never depend on each other or on the bindings.

## Shared abstractions

- `Scalar` is `f64` everywhere. See conventions for why.
- `Vec2` with the handful of operations mechanics needs. No general linear algebra dependency in the core.
- `Invariant { name, value }` for conserved quantities, reported by every simulation.
- `Event { time, kind, subjects }` for discrete happenings: a contact begins, a rope goes slack, friction switches from
  static to kinetic. Events are part of the physics output, not logging.
- `Recorder` samples a simulation into columnar time series that bindings hand to NumPy or JavaScript without copying
  into per-row objects.
- `Simulation` trait: `time()`, `dt()`, `step()`, `advance(n)`, `invariants()`. Bindings and the playground loop drive
  anything through this trait.

## Rules every solver obeys

1. A fixed time step chosen by the user. No hidden adaptivity. Reproducible to the bit across runs and platforms.
2. Conserved quantities are computed by the model, not estimated by the driver, and are always available.
3. Every scenario in the catalogue ships with a closed-form answer or a convergence study, and a test that checks it.
4. Reactions are exposed. If a constraint or contact pushes on a body, the force it applied is reported with a label.
5. The engine never silently clamps, damps or fudges. If a stabilisation term exists, it is named, documented and off by
   default unless the design document says otherwise.
6. Units are SI. Gravity defaults to 9.8 m/s², matching NCERT, and is configurable.

## Data flow

```
scene (JSON or builder)  ->  World  ->  step loop  ->  snapshot / Recorder  ->  NumPy arrays | JS typed arrays
                                   ^                        |
                                   +---- parameter edits ---+   (playground sliders rebuild the world)
```

A scene is data. The playground and a notebook can load the same file and get the same numbers.
