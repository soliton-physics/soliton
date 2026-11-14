# Design documents

Every significant piece of the engine starts here as a short design document before it becomes code.
A document states the physics, the numerical method, the public API, and how the result is validated.

| Status | Meaning |
| --- | --- |
| Draft | under discussion, nothing depends on it |
| Accepted | agreed; implementation may start |
| Implemented | code and tests exist; the document is the reference |
| Superseded | replaced by a later document |

## Standing documents

- [architecture.md](architecture.md): crates, the two pillars, shared abstractions, rules every solver obeys
- [conventions.md](conventions.md): naming, layout, comments, testing and validation policy
- [roadmap.md](roadmap.md): phases and what each one delivers

## Numbered designs

| # | Title | Status |
| --- | --- | --- |
| [001](001-mechanics-core.md) | Mechanics core: world, bodies, forces, integrators, recorder | Implemented |
| [002](002-constraints-and-ropes.md) | Bilateral constraints, pin joints, ropes and pulleys | Implemented |
| [003](003-contact-and-friction.md) | Contact, Coulomb friction, restitution, time stepping | Implemented |
| [004](004-scenes-and-scenarios.md) | Scene format, parameters, the NCERT scenario catalogue | Implemented |
| [005](005-python-and-jupyter.md) | Python package and notebook workflow | Implemented (first slice) |
| [006](006-playground-protocol.md) | What the WebAssembly layer exposes to the playground | Implemented |
| [007](007-fields-baseline.md) | Spectral field solvers: what exists and what follows | Implemented (baseline) |
| [008](008-interaction.md) | A hand on the physics: grabbing, picking, snapshots | Implemented |
| [009](009-gravitation-and-oscillators.md) | Newtonian gravitation, orbits, damped and driven oscillators | Implemented |
| [010](010-class-11-catalogue.md) | The rest of the class 11 catalogue; a free-ended spring; what counts as an impact | Implemented |
| [011](011-electromagnetism.md) | Charges and fields for class 12: Coulomb's law, uniform electric and magnetic fields | Implemented |
| [012](012-magnetism-and-matter.md) | Gauss's law and fixed spreads of charge; bar magnets, the earth's field, magnetic materials | Implemented |
| [013](013-ray-optics.md) | Ray optics: exact tracing through mirrors, glass and thin lenses; paraxial images; fifteen scenarios | Implemented |
| [014](014-thermal.md) | Thermal physics: a gas of hard spheres, pistons, heat baths, staged experiments; nine scenarios | Implemented |
| [015](015-circuits.md) | Circuits from Kirchhoff's laws: steady, transient and alternating; nineteen scenarios | Implemented |
| [016](016-statistical-mechanics.md) | Monte Carlo and the Ising model: darts, random walks, Metropolis, a temperature sweep | Implemented |
