# soliton

Python bindings for the [Soliton Physics](https://github.com/soliton-physics/soliton) engine.

```python
import numpy as np
from soliton import Simulation, kdv_soliton

sim = Simulation("kdv", points=512, length=60.0, dt=1e-3)
sim.state = kdv_soliton(sim.x, t=0.0, speed=4.0, center=-10.0)
sim.step(5000)
print(sim.time, sim.invariants())
```

Build locally with `maturin develop --release` from this directory.
