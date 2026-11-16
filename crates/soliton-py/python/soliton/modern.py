"""Modern physics: photons, matter waves, atoms and nuclei.

    from soliton import modern

    modern.electron_wavelength(54)            # 0.167 nm
    modern.line_wavelength(3, 2)              # 656 nm, the red line of hydrogen
    modern.alpha_deflection(5.5, 79, 30)      # degrees, by integrating the path past a gold nucleus

    run = modern.scenario("radioactive-decay", half_life=4, nuclei=2500)
    while not run.finished:
        run.advance(0.05)                     # seconds
    run.answers(), run.measured()             # N0/2, N0/4 ...; what this sample did

Energies of photons and atoms are in eV, of nuclei in MeV; lengths in nm for atoms and fm for nuclei.
"""

import json

from soliton._native import ModernRun as Run
from soliton._native import alpha_deflection, bohr_energy, electron_wavelength, line_wavelength, liquid_drop, modern_catalogue_json

__all__ = ["Run", "catalogue", "scenario", "electron_wavelength", "bohr_energy", "line_wavelength", "alpha_deflection", "liquid_drop"]


def catalogue() -> list[dict]:
    """Every modern physics scenario: id, title, description and parameter schema."""
    return json.loads(modern_catalogue_json())


def scenario(id: str, **params: float) -> Run:
    """A catalogue scenario with chosen parameters; unset parameters take their defaults."""
    return Run.scenario(id, **params)
