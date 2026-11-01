# 005: Python package and notebook workflow

Status: Draft

## Goals

- A teacher or student writes five lines in a notebook and gets arrays they can plot with Matplotlib.
- A researcher gets the same engine, the same numbers, and columnar NumPy output without copies.
- Everything a scene can express in JSON can be expressed in Python, and round-trips.

## Package layout

```
soliton/
  __init__.py          re-exports both pillars
  mechanics/           World, bodies, forces, constraints, Scene, scenarios, Run
  fields/              today's Simulation, kdv_soliton, nls_bright_soliton
  _native              the PyO3 extension
```

## The notebook experience

```python
from soliton.mechanics import World, Particle, HalfPlane, Applied, Recorder

world = World(gravity=(0, -9.8))
floor = world.add_static(HalfPlane(normal=(0, 1), offset=0), friction=(0.4, 0.3))
block = world.add(Particle(mass=2.0, position=(0, 0)))
world.add(Applied(block, force=(10, 0), until=2.0))

run = world.run(until=5.0, dt=1e-3, record_every=10)

run.t                       # ndarray
run[block].x, run[block].v  # ndarray (N,) and (N, 2)
run[block].force("friction")  # ndarray (N, 2); static friction appears here with its real value
run.energy.total, run.energy.dissipated
run.events                  # [Event(t=0.000, kind="contact_begin", ...), Event(t=0.613, kind="friction_slip", ...)]
```

And the catalogue:

```python
from soliton.mechanics import scenarios

scene = scenarios.atwood(m1=2, m2=3)
scene.exact()               # {"acceleration": 1.96, "tension": 23.52}
run = scene.run(until=2.0)
```

`run.to_dataframe()` is available when pandas is installed. Plotting stays in the notebook; the package does not
depend on Matplotlib.

## Live widgets

A small `anywidget` front end that reuses the playground's renderer lets a notebook show the animation inline, with
sliders generated from the scenario's parameter schema. It ships after the playground stabilises, so the two share
one renderer.

## Packaging

- `maturin` builds abi3 wheels for Linux, macOS and Windows in CI; published to PyPI as `soliton`.
- NumPy is the only runtime dependency.
- Versioned together with the Rust crates.
