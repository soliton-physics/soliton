"""Currents and magnetism: the field a current makes, the force on a current, electromagnetic induction.

    from soliton import magnetism

    run = magnetism.scenario("motional-emf", pull=0.1)          # a rod pulled along rails in a field
    while not run.finished:
        run.advance(0.02)                                       # seconds of watching
    run.answers(), run.measured()                               # terminal speed FR/B²l², both ways

    magnetism.scenario("straight-wire-field", current=5).readings()   # a law evaluated: nothing to play

Fields are summed from the Biot–Savart law and every emf is found from the change of flux, so `measured()` is
an independent check on each formula in `answers()`. SI units, with the textbook's prefixes shown in each label.
"""

import json

from soliton._native import MagnetismRun as Run
from soliton._native import magnetism_catalogue_json

__all__ = ["Run", "catalogue", "scenario"]


def catalogue() -> list[dict]:
    """Every currents-and-magnetism scenario: id, title, description and parameter schema."""
    return json.loads(magnetism_catalogue_json())


def scenario(id: str, **params: float) -> Run:
    """A catalogue scenario with chosen parameters; unset parameters take their defaults."""
    return Run.scenario(id, **params)
