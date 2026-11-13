"""Planar mechanics: the NCERT scenario catalogue and JSON scenes (designs 004 and 005).

    from soliton import mechanics

    scene = mechanics.scenario("atwood", m1=2, m2=3)
    scene.answers()                      # {'Acceleration (m/s²)': 1.96, 'Tension (N)': 23.52}
    run = scene.run(until=2.0)
    run.t, run.body("m2")["velocity"]    # NumPy arrays
    run.force("m2", "tension")           # (N, 2)
    scene.save("atwood.json")            # the same file the playground loads

Class 12 scenes carry charges and fields (design 011):

    scene = mechanics.scenario("field-of-two-charges")
    scene.electric_field(0.0, 1.0)       # (Ex, Ey) in N/C
    scene.electric_potential(0.0, 1.0)   # volts

and bar magnets (design 012):

    scene = mechanics.scenario("bar-magnet-field")
    scene.magnetic_field(0.2, 0.0)       # (Bx, By) in tesla
    mechanics.scenario("gauss-law").electric_flux(-1.0, 0.0, 0.7)   # N·m²/C
"""

import json

from soliton._native import Run, Scene, catalogue_json

__all__ = ["Scene", "Run", "catalogue", "scenario", "load"]


def catalogue() -> list[dict]:
    """Every scenario: id, title, description, syllabus and parameter schema."""
    return json.loads(catalogue_json())


def scenario(id: str, **params: float) -> Scene:
    """A catalogue scenario with chosen parameters; unset parameters take their defaults."""
    return Scene.scenario(id, **params)


def load(path: str) -> Scene:
    """A scene from a JSON file, as saved by `Scene.save` or exported from the playground."""
    return Scene.load(path)
