"""Electric circuits, solved from Kirchhoff's laws.

    from soliton import circuits

    bridge = circuits.scenario("wheatstone-bridge", s=30)
    bridge.current("G")                  # amperes: zero, it is balanced
    bridge.answers(), bridge.measured()

    rc = circuits.scenario("charging-capacitor")
    rc.advance(1.0)                      # seconds
    t, v, i = rc.series("C")

    lcr = circuits.scenario("series-lcr", frequency=35.6)
    V, I = lcr.phasor("R")               # complex amplitudes

A component's voltage is that of its `a` end above its `b` end, and its current runs from `a` to `b` through it;
so a cell that is supplying current has a negative one. Nothing about series, parallel, bridges or impedance is
built in: the same equations are solved for steady currents, step by step in time, and with complex numbers for
alternating current.
"""

import json

from soliton._native import Circuit, circuits_catalogue_json

__all__ = ["Circuit", "catalogue", "scenario"]


def catalogue() -> list[dict]:
    """Every circuit scenario: id, title, description, syllabus and parameter schema."""
    return json.loads(circuits_catalogue_json())


def scenario(id: str, **params: float) -> Circuit:
    """A catalogue scenario with chosen parameters; unset parameters take their defaults."""
    return Circuit.scenario(id, **params)
