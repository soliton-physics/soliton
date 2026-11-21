"""Properties of matter: solids under stress, fluids at rest and in flow, heat and temperature.

    from soliton import matter

    run = matter.scenario("buoyancy", block=917, liquid=1030)   # ice in sea water
    while not run.finished:
        run.advance(0.02)                                       # seconds
    run.answers(), run.measured()                               # 89 % under the surface, both ways

    matter.scenario("stress-strain", load=10).readings()        # a law evaluated: nothing to play

SI units throughout, with the prefixes the textbook uses shown in each label.
"""

import json

from soliton._native import MatterRun as Run
from soliton._native import matter_catalogue_json

__all__ = ["Run", "catalogue", "scenario"]


def catalogue() -> list[dict]:
    """Every properties-of-matter scenario: id, title, description and parameter schema."""
    return json.loads(matter_catalogue_json())


def scenario(id: str, **params: float) -> Run:
    """A catalogue scenario with chosen parameters; unset parameters take their defaults."""
    return Run.scenario(id, **params)
