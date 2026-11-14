"""Statistical mechanics by Monte Carlo: the Ising model, and the simplest random experiments.

    from soliton import statmech

    run = statmech.scenario("ising-model", temperature=2.0, size=48)
    run.advance(2000)                    # sweeps
    run.spins()                          # (48, 48) array of +1 and -1
    run.answers(), run.measured()        # Onsager's exact values, and the averages found

    sweep = statmech.scenario("phase-transition")
    while not sweep.finished:
        sweep.advance(500)
    T, m, e, c, chi = sweep.points().T

    statmech.onsager_magnetisation(2.0)  # 0.9113...
    statmech.CRITICAL_TEMPERATURE        # 2/ln(1 + sqrt 2)

Temperatures are in units of J/k and energies in units of J, the coupling between neighbouring spins.
"""

import json
import math

from soliton._native import StatRun as Run
from soliton._native import onsager_energy, onsager_magnetisation, statmech_catalogue_json

CRITICAL_TEMPERATURE = 2.0 / math.log(1.0 + math.sqrt(2.0))

__all__ = ["Run", "catalogue", "scenario", "onsager_magnetisation", "onsager_energy", "CRITICAL_TEMPERATURE"]


def catalogue() -> list[dict]:
    """Every statistical scenario: id, title, description and parameter schema."""
    return json.loads(statmech_catalogue_json())


def scenario(id: str, **params: float) -> Run:
    """A catalogue scenario with chosen parameters; unset parameters take their defaults."""
    return Run.scenario(id, **params)
