# Roadmap

Phases are ordered by who they serve. Each phase ends with something a real person can use.

## Phase 1: Mechanics for class 11 (teachers can try it)

Goal: a teacher opens the playground or a notebook and runs every mechanics scenario in the NCERT class 11 syllabus,
with textbook answers reproduced and free-body diagrams that show the engine's own numbers.

- Mechanics core: bodies, forces, velocity Verlet, recorder, energies and momenta (001)
- Pin joints, rods, ropes over ideal pulleys (002)
- Contact with Coulomb friction and restitution against planes, circles and polygons (003)
- Scene format and the scenario catalogue with validation (004)
- Python package with NumPy output and notebook examples (005)
- Playground: scenario picker, parameter sliders, vectors, free-body diagram, graphs (006)

Exit: every row in the catalogue table passes its closed-form test; three teachers have used it.

## Phase 2: Rotation, gravitation, class 12 fields

- Massive pulleys with no-slip rope, flywheels, rolling that emerges from friction
- Newtonian gravity between bodies: orbits, Kepler's laws, escape velocity
- Charged particles in uniform electric and magnetic fields: cyclotron motion, velocity selector (class 12)
- Oscillations beyond the basics: damped, driven, resonance curves, coupled oscillators
- Event timing by interpolation so reported impact and landing times are accurate to second order

## Phase 3: Waves and the fields pillar, continued

- Sine-Gordon and second-order field equations
- Dealiasing, filters, adaptive stepping
- A chain of masses and springs that becomes a wave on a string: the bridge between the two pillars
- Two-dimensional fields, WebGPU rendering

## Phase 4: Research tooling

- PyPI and npm releases, versioned scene schema
- Inverse scattering diagnostics for the wave solvers
- Discrete adjoints for parameter fitting
