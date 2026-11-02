<p align="center">
  <img src="https://raw.githubusercontent.com/soliton-physics/.github/main/logo/soliton.svg" width="120" alt="Soliton Physics">
</p>

<h1 align="center">soliton</h1>

<p align="center">
  Research-grade, educational physics engine. Rust core, Python bindings, WebAssembly for the browser.
</p>

## What this is

`soliton` is the engine behind [Soliton Physics](https://github.com/soliton-physics): a set of validated numerical solvers
written once in Rust and exposed to three audiences from the same code.

- **Researchers** use the Python package (`soliton`) with NumPy arrays, Jupyter and SciPy.
- **Learners** use the browser [playground](https://github.com/soliton-physics/playground), which runs this engine compiled to WebAssembly.
- **Readers** follow the [docs](https://github.com/soliton-physics/docs), where every explanation sits next to a live simulation.

The first vertical is nonlinear waves on periodic domains, solved with Fourier pseudo-spectral methods and
fourth-order exponential time differencing (ETDRK4).

| Equation | Form | Status |
|---|---|---|
| Korteweg–de Vries | `u_t + 6 u u_x + u_xxx = 0` | validated against exact solitons |
| Nonlinear Schrödinger (focusing) | `i ψ_t + ½ ψ_xx + |ψ|² ψ = 0` | validated against exact bright solitons |

Every solver ships with tests that compare against closed-form solutions and check conserved quantities.
Current accuracy on the reference tests: KdV soliton error `5e-8`, NLS soliton error `3e-9`, invariant drift below `1e-8`.

## Layout

```
crates/
  soliton-core        shared types: Vec2, Invariant, the Simulate trait
  soliton-mechanics   planar rigid-body mechanics: bodies, forces, constraints, contact
  soliton-fields      spectral solvers for wave equations: grids, FFT, ETDRK4, exact solutions
  soliton-wasm        wasm-bindgen bindings consumed by the playground and docs
  soliton-py          PyO3 bindings packaged with maturin as the `soliton` Python package
```

## Rust

```rust
use soliton_fields::exact::kdv_soliton;
use soliton_fields::{Complex64, Kdv, PeriodicGrid, Simulate, Simulation};

let mut sim = Simulation::new(PeriodicGrid::new(512, 60.0), Kdv, 1e-3);
sim.set_profile(|x| Complex64::new(kdv_soliton(x, 0.0, 4.0, -10.0), 0.0));
sim.advance(5000);
println!("t = {}, invariants = {:?}", sim.time(), sim.invariants());
```

```sh
cargo test --release
```

## Python

```sh
cd crates/soliton-py
uv venv && source .venv/bin/activate
uv pip install maturin numpy
maturin develop --release
```

```python
import numpy as np
from soliton import Simulation, kdv_soliton

sim = Simulation("kdv", points=512, length=60.0, dt=1e-3)
sim.state = kdv_soliton(sim.x, 0.0, speed=4.0, center=-10.0)
sim.step(5000)
print(sim.time, sim.invariants())
```

## WebAssembly

```sh
cargo install wasm-pack
cd crates/soliton-wasm
wasm-pack build --release --target web --scope soliton-physics
```

The package lands in `crates/soliton-wasm/pkg`. The playground vendors it with `pnpm sync-engine`.

## Design

- One `SemiLinear` trait describes an equation `u_t = L u + N(u)`: a Fourier symbol for `L` and a
  physical-space nonlinearity `N`. Adding an equation is one small file.
- Steppers are generic over equations. ETDRK4 precomputes its coefficients by a full-circle contour mean,
  which keeps them exact for complex linear symbols such as dispersion.
- Conserved quantities are part of the equation definition, so every simulation can report its own drift.

## Releases

Tag `vX.Y.Z` on `main` and the release workflow publishes `soliton-physics` to PyPI (abi3 wheels for Linux, macOS
and Windows) and `@soliton-physics/engine` to npm. Both use trusted publishing through GitHub's OIDC token, so the
repository holds no registry credentials.

## Design and roadmap

The engine is designed in the open. [`design/`](design/) holds the architecture, conventions, the phased roadmap, and one
design document per subsystem. Phase 1 is planar mechanics for the NCERT class 11 syllabus: bodies, forces, constraints,
ropes and pulleys, contact with Coulomb friction, all validated against textbook answers. The wave solvers above are the
first piece of the fields pillar and continue after that.

## License

Dual-licensed under MIT or Apache-2.0.
