# soliton-physics

Python bindings for the [Soliton Physics](https://github.com/soliton-physics/soliton) engine: `pip install soliton-physics`, then `import soliton`.

```python
from soliton import mechanics

scene = mechanics.scenario("atwood", m1=2, m2=3)
scene.answers()
run = scene.run(until=2.0)
run.body("m2")["velocity"]
```

The wave solvers:

```python
import numpy as np
from soliton import Simulation, kdv_soliton

sim = Simulation("kdv", points=512, length=60.0, dt=1e-3)
sim.state = kdv_soliton(sim.x, t=0.0, speed=4.0, center=-10.0)
sim.step(5000)
print(sim.time, sim.invariants())
```

Build locally with `maturin develop --release` from this directory.
