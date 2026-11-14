"""Thermal physics: a gas as molecules in a box, with pistons and heat reservoirs.

    from soliton import thermal

    gas = thermal.scenario("adiabatic-process", temperature=300)
    while not gas.finished and gas.stage < 2:
        gas.advance(50)                 # picoseconds
    gas.temperature                     # K, after the compression
    gas.answers()                       # what the textbook says
    gas.measured()                      # what the molecules gave
    v = gas.velocities()                # (N, 3) in m/s
    gas.series()["pressure"]            # atm, once a picosecond

Nothing thermodynamic is put in. Temperature is the molecules' mean kinetic energy, pressure is counted from
their blows on the walls, work is what the moving piston gives them and heat what a wall held at a temperature
hands over. Lengths are in nanometres, times in picoseconds.
"""

import json

from soliton._native import Gas, thermal_catalogue_json

__all__ = ["Gas", "catalogue", "scenario"]


def catalogue() -> list[dict]:
    """Every thermal scenario: id, title, description, syllabus and parameter schema."""
    return json.loads(thermal_catalogue_json())


def scenario(id: str, **params: float) -> Gas:
    """A catalogue scenario with chosen parameters; unset parameters take their defaults."""
    return Gas.scenario(id, **params)
